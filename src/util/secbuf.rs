use std::fs::{OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::{io::Read, iter, marker::PhantomData};

use age::{Identity, Recipient};
#[derive(Debug, Clone)]
pub struct AgeEnc;
#[derive(Debug, Clone)]
pub struct HostEnc;
#[derive(Debug, Clone)]
pub struct Plain;

#[derive(Debug, Clone)]
pub struct SecBuf<T> {
    buf: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T> SecBuf<T> {
    pub fn new(i: Vec<u8>) -> Self {
        SecBuf {
            buf: i,
            _marker: PhantomData,
        }
    }
    pub fn inner(self) -> Vec<u8> {
        self.buf
    }
    pub fn inner_ref(&self) -> &Vec<u8> {
        self.buf.as_ref()
    }

    pub fn hash_with(&self, host_ssh_recip: &str) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.buf);
        hasher.update(host_ssh_recip.as_bytes());
        hasher.finalize()
    }
}

use eyre::Result;
impl<T> SecBuf<T> {
    pub fn buf_ref(&self) -> &Vec<u8> {
        self.buf.as_ref()
    }
}

pub trait Decryptable {
    fn decrypt(&self, ident: &dyn Identity) -> Result<SecBuf<Plain>>;
}

macro_rules! impl_decryptable {
    ($type:ty) => {
        impl Decryptable for $type {
            fn decrypt(&self, ident: &dyn Identity) -> Result<SecBuf<Plain>> {
                let buffer = self.buf_ref();
                let decryptor = age::Decryptor::new(&buffer[..])?;

                let mut dec_content = vec![];
                let mut reader = decryptor.decrypt(iter::once(ident))?;
                let res = reader.read_to_end(&mut dec_content);
                if let Ok(b) = res {
                    debug!("decrypted secret {} bytes", b);
                }
                Ok(SecBuf::new(dec_content))
            }
        }
    };
}

impl_decryptable!(SecBuf<HostEnc>);
impl_decryptable!(SecBuf<AgeEnc>);

impl<T> From<Vec<u8>> for SecBuf<T> {
    fn from(value: Vec<u8>) -> Self {
        Self {
            buf: value,
            _marker: PhantomData,
        }
    }
}

impl SecBuf<AgeEnc> {
    #[cfg(test)]
    pub fn renc<'a>(
        &self,
        ident: &dyn Identity,
        recips: impl Iterator<Item = &'a (dyn Recipient + Send)>,
    ) -> Result<SecBuf<HostEnc>> {
        self.decrypt(ident).and_then(|d| d.encrypt(recips))
    }
}

use eyre::eyre;
use log::{debug, trace};

use crate::parser::extract_all_hashes;
use crate::profile::InsertSet;

use super::set_owner_group;

impl SecBuf<Plain> {
    /// encrypt with host pub key, ssh key
    pub fn encrypt<'a>(
        self,
        recips: impl Iterator<Item = &'a (dyn Recipient + Send)>,
    ) -> Result<SecBuf<HostEnc>> {
        let recips = recips.map(|r| r as &dyn Recipient);
        let encryptor =
            age::Encryptor::with_recipients(recips).map_err(|_| eyre!("create encryptor err"))?;

        let buf = self.buf_ref();
        let mut enc_content = vec![];

        let mut writer = encryptor.wrap_output(&mut enc_content)?;

        use std::io::Write;
        writer.write_all(buf)?;
        writer.finish()?;
        Ok(SecBuf::new(enc_content))
    }

    pub fn deploy_to_fs(
        &self,
        item: impl crate::profile::DeployFactor,
        dst: PathBuf,
    ) -> Result<()> {
        let mut the_file = {
            let mode = crate::parser::parse_permissions_str(item.mode())
                .map_err(|e| eyre!("parse octal permission err: {}", e))?;
            let permissions = Permissions::from_mode(mode);
            trace!("apply file permission: {permissions:?}");

            let file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(dst)?;

            file.set_permissions(permissions)?;

            set_owner_group::set_owner_and_group(&file, item.owner(), item.group())?;

            file
        };
        the_file.write_all(self.buf_ref())?;
        Ok(())
    }

    pub fn insert(&mut self, ins_set: &InsertSet, clean_after_replace_complete: bool) {
        let ins_map = &ins_set.0;
        let mut hash_extract_res = vec![];

        let self_string = String::from_utf8(self.inner_ref().clone()).expect("must");

        extract_all_hashes(self_string.as_str(), &mut hash_extract_res);

        log::trace!("{:?}", &hash_extract_res);

        let mut ins_map: Vec<_> = ins_map.iter().collect();

        ins_map.sort_by_key(|(_, v)| v.order);

        let mut new_string = self_string.clone();

        let brace_the_str = |s: &str| -> String { format!("{{{{ {s} }}}}") };

        ins_map.iter().for_each(|(k, v)| {
            if hash_extract_res.contains(&k.as_str()) {
                log::debug!("inserting content corresponding to placeholder: {k}");
                let braced_hash_str = brace_the_str(k);
                let string_after_this_replace =
                    new_string.replace(braced_hash_str.as_str(), &v.content);
                new_string = string_after_this_replace;
                hash_extract_res.retain(|&x| x != k.as_str());
            } else {
                log::error!(
                    "corresponding content of existing placeholder not found in `insert`: {k}"
                );
            }
        });
        if clean_after_replace_complete {
            hash_extract_res.iter().for_each(|i| {
                let braced_hash_str = brace_the_str(i);
                let string_after_clean =
                    new_string.replace(braced_hash_str.as_str(), String::default().as_str());
                new_string = string_after_clean;
            });
        }
        *self = SecBuf::<Plain>::new(new_string.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, str::FromStr};

    use nom::AsBytes;

    use super::*;

    #[test]
    fn test_renc() {
        let key = age::x25519::Identity::generate();
        let pubkey = key.to_public();

        let plaintext = b"Hello world!";

        // Encrypt the plaintext to a ciphertext...
        let encrypted = {
            let encryptor = age::Encryptor::with_recipients(iter::once(&pubkey as _))
                .expect("we provided a recipient");

            let mut encrypted = vec![];
            let mut writer = encryptor.wrap_output(&mut encrypted).expect("test");
            writer.write_all(plaintext).expect("test");
            writer.finish().expect("test");

            encrypted
        };

        // 0x01
        let new_recip_str = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
        let buf = SecBuf::<AgeEnc>::new(encrypted);
        let r =
            &age::x25519::Recipient::from_str(new_recip_str).unwrap() as &(dyn Recipient + Send);

        let boxed_key: Box<dyn Identity> = Box::new(key);

        let _ = buf.renc(boxed_key.as_ref(), iter::once(r)).unwrap();
    }

    #[test]
    fn b3_hex_decode() {
        let _ = blake3::Hash::from_hex(
            hex::decode("21634884238b81de20c58fab119c9e52922c57256234b3658db81c7b9e1f71d5")
                .unwrap()
                .as_bytes(),
        );
    }
}
