use std::{
    collections::HashMap,
    fs::{self, Permissions, ReadDir},
    io::{self, ErrorKind},
    iter,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
};

use crate::{
    cmd::renc::CompleteProfile,
    profile::{DeployFactor, HostKey, Profile, Secret},
    util::{
        secbuf::{Plain, SecBuf},
        secmap::{RencBuilder, RencCtx},
    },
};

use crate::parser::recipient::RawRecip;
use age::{Identity, Recipient};
use eyre::{Context, ContextCompat, Result, bail, eyre};
use hex::decode;
use lib::extract_all_hashes;
use log::{debug, error, info};
use strum_macros::Display;
use sys_mount::{Mount, MountFlags, SupportedFilesystems};

impl HostKey {
    pub fn get_identity(&self) -> Result<age::ssh::Identity> {
        fs::read_to_string(&self.path)
            .wrap_err_with(|| eyre!("reading ssh host key error: {}", self.path))
            .and_then(|i| {
                age::ssh::Identity::from_buffer(i.as_bytes(), None)
                    .map_err(|e| eyre!("convert age identity from ssh key error: {}", e))
            })
    }
}

macro_rules! impl_get_settings {
    ([ $($field:ident),+ $(,)? ]) => {
        impl Profile {
            $(
                pub fn $field(&self) -> &str {
                    self.settings.$field.as_str()
                }
            )+
        }
    };
}

impl_get_settings!([
    decrypted_mount_point,
    decrypted_dir,
    decrypted_dir_for_user,
    host_identifier,
    host_pubkey
]);

// key type corresponding to type of element of `services.openssh.hostKeys` in nixos module
#[derive(Display)]
#[strum(serialize_all = "lowercase")]
enum ModKeyType {
    Rsa,
    ED25519,
}

impl Profile {
    pub fn read_decrypted_mount_point(&self) -> std::io::Result<ReadDir> {
        fs::read_dir(self.decrypted_mount_point())
    }

    // TODO: try iterate the list of key met type
    pub fn get_host_key_identitys(&self) -> Result<Vec<age::ssh::Identity>> {
        let ssh_key_type =
            if age::ssh::Recipient::from_str(self.settings.host_pubkey.as_str()).is_ok() {
                ModKeyType::ED25519.to_string()
            } else {
                // pretend. since age only support these two
                ModKeyType::Rsa.to_string()
            };

        debug!("determined host ssh key type: {}", &ssh_key_type);

        let ret = self
            .settings
            .host_keys
            .iter()
            .filter_map(|k| {
                if k.r#type == ssh_key_type.as_str() {
                    debug!("found host private key that matches type: {:?}", k);
                    match k.get_identity() {
                        Ok(o) => return Some(o),
                        Err(e) => {
                            error!("parse identity error: {e}");
                            return None;
                        }
                    }
                }
                None
            })
            .collect::<Vec<age::ssh::Identity>>();
        if ret.is_empty() {
            bail!(
                "no parsed host key matches the recipient type: {}",
                &ssh_key_type
            )
        }
        Ok(ret)
    }

    pub fn _get_host_recip(&self) -> Result<Box<dyn Recipient + Send>> {
        let recip: RawRecip = self.settings.host_pubkey.clone().into();
        recip.try_into()
    }

    /// init decrypted mount point and return the generation count
    pub fn init_decrypted_mount_point(&self) -> Result<usize> {
        let mut max = 0;
        let res = match self.read_decrypted_mount_point() {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                let support_ramfs =
                    SupportedFilesystems::new().map(|fss| fss.is_supported("ramfs"));
                if !support_ramfs? {
                    let err =
                        "ramfs not supported! Refusing extract secret since it will write to disk";
                    error!("{}", err);
                    eyre::bail!(err);
                }
                let path = self.decrypted_mount_point();
                info!("creating mount point {}", path);
                fs::create_dir_all(path).wrap_err_with(|| {
                    format!(
                        "creating decrypted mountpoint: {:?}",
                        self.decrypted_mount_point()
                    )
                })?;
                Mount::builder()
                    .fstype("ramfs")
                    .flags(MountFlags::NOSUID)
                    .data("relatime")
                    .data("mode=751")
                    .mount(String::default(), self.decrypted_mount_point())
                    .map(|_| ()) // not needed.
                    .wrap_err(eyre!("mount tmpfs error"))
            }
            Err(e) => {
                error!("{}", e);
                Err(e).wrap_err(eyre!("read mountpoint error"))
            }
            Ok(ref mut o) => o.try_for_each(|en| {
                en.wrap_err_with(|| eyre!("enter secret mount point error"))
                    .and_then(|d| {
                        match str::parse::<usize>(
                            d.file_name().to_string_lossy().to_string().as_str(),
                        ) {
                            Err(e) => Err(eyre!("parse mount point generation err: {}", e)),
                            Ok(res) => {
                                debug!("found mountpoint generation {}", res);
                                if res >= max {
                                    max = res + 1;
                                }
                                Ok(())
                            }
                        }
                    })
            }),
        };

        res.map(|_| max)
    }
    /**
    extract secrets to `/run/vaultix.d/$num` and link to `/run/vaultix`
    */
    pub fn deploy(&self, early: bool) -> Result<()> {
        if self.secrets.is_empty() && self.templates.is_empty() {
            info!("nothing needs to deploy. finish");
            return Ok(());
        }
        if self.before_userborn.is_empty() && early {
            info!("nothing needs to deploy before userborn. finish");
            return Ok(());
        }

        let host_prv_keys = if let Ok(o) = self.get_host_key_identitys() {
            let mut v: Vec<Box<dyn Identity>> = vec![];
            for k in o.into_iter() {
                v.push(Box::new(k))
            }
            v
        } else {
            bail!("no identity parsed")
        };

        let if_early = |i: &String| -> bool { self.before_userborn.contains(i) == early };

        let secrets = self.secrets.values().filter(|i| if_early(&i.id));

        let templates = self.templates.iter().filter(|i| if_early(i.0));

        let symlink_dst = if early {
            self.decrypted_dir_for_user()
        } else {
            self.decrypted_dir()
        };

        match fs::symlink_metadata(symlink_dst) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            e @ Err(_) => e.map(|_| ())?,
            Ok(res) => {
                if !res.file_type().is_symlink() {
                    bail!("The decrypted dir {symlink_dst} should be symlink. abort")
                }
            }
        }

        let complete = CompleteProfile::from_iter(iter::once(self));

        let ctx = RencCtx::create(&complete)?;

        let plain_map: HashMap<&Secret, Vec<u8>> = RencBuilder::create(&complete)
            .build_instore()
            .renced_stored(&ctx, self.settings.cache_in_store.clone().into())
            .bake_decrypted(host_prv_keys)
            .wrap_err_with(|| eyre!("decrypt failed, please delete cache dir and try re-encrypt"))
            .and_then(|i| {
                i.into_iter()
                    .map(|(k, v)| {
                        let ins_set = &k.insert.0;
                        if !ins_set.is_empty() || k.clean_placeholder {
                            let mut plain = SecBuf::<Plain>::new(v);
                            plain.insert(&k.insert, k.clean_placeholder);
                            Ok((k, plain.inner()))
                        } else {
                            Ok((k, v))
                        }
                    })
                    .collect()
            })?;

        let generation = self.init_decrypted_mount_point()?;

        let target_extract_dir_with_gen = {
            let mut p = PathBuf::from(self.decrypted_mount_point());
            p.push(generation.to_string());

            debug!("target extract dir with generation number: {:?}", p);

            fs::create_dir_all(&p)
                .map(|_| p)
                .wrap_err(eyre!(
                    "cannot create target extract dir with generation number"
                ))
                .inspect(|p| {
                    fs::set_permissions(p, Permissions::from_mode(0o751))
                        .wrap_err(eyre!("set permission failed"))
                        .expect("permission issue");
                })?
        };
        macro_rules! generate_dst {
            ($obj:expr, $settings:expr, $target_extract_dir:expr) => {{
                let default_path = {
                    let mut p: PathBuf = $settings.decrypted_dir.clone().into();
                    p.push($obj.name());
                    p
                };
                if PathBuf::from($obj.path()) == default_path {
                    let mut ret = $target_extract_dir.clone();
                    ret.push($obj.name());
                    ret
                } else {
                    if PathBuf::from($obj.path()).starts_with(&default_path) {
                        log::warn!(
                            "extract to decryptedDir detected. recommend specify `name` instead of `path`."
                        );
                    }
                    info!("specified decrypt path detected");
                    $obj.path().into()
                }
            }};
        }

        // deploy general secrets
        secrets
            .map(|n| {
                let raw_content = plain_map
                    .get(n)
                    .wrap_err_with(|| eyre!("decrypted content must found"))?;
                let plain = SecBuf::<Plain>::new(raw_content.clone());

                let item = &n as &dyn DeployFactor;
                let dst: PathBuf = generate_dst!(item, self.settings, target_extract_dir_with_gen);

                info!("secret {} -> {}", item.name(), dst.display(),);

                plain.deploy_to_fs(n, dst)
            })
            .for_each(|res| {
                if let Err(e) = res {
                    error!("{}", e);
                }
            });
        info!("finish secrets deployment");

        if !self.templates.is_empty() {
            info!("start templates deployment");
            // new map with {{ hash }} String as key, content as value
            let hashstr_content_map: std::collections::HashMap<&str, &Vec<u8>> = plain_map
                .iter()
                .map(|(k, v)| {
                    self.placeholder
                        .get_braced_from_id(k.id.as_str())
                        .wrap_err_with(|| {
                            eyre!("secrets corresponding to the template placeholder id not found")
                        })
                        .map(|i| (i, v))
                        .expect("found secret from placeholder id")
                })
                .collect();

            templates
                .map(|(_, t)| {
                    let mut template = t.content.clone();
                    let hashstrs_of_it = t.parse_hash_str_list().expect("parse template");

                    let trim_the_insertial = t.trim;

                    hashstr_content_map
                        .iter()
                        .filter(|(k, _)| {
                            let mut v = Vec::new();
                            extract_all_hashes(k, &mut v);
                            hashstrs_of_it
                                // promised by nixos module
                                .contains(&decode(v.first().expect("only one")).expect("decoded"))
                        })
                        .for_each(|(k, v)| {
                            // render and insert
                            log::trace!("template before process: {}", template);

                            let raw_composed_insertial = String::from_utf8_lossy(v).to_string();

                            let insertial = if trim_the_insertial {
                                raw_composed_insertial.trim()
                            } else {
                                raw_composed_insertial.as_str()
                            };

                            template = template.replace(k, insertial);
                        });

                    let item = &t as &dyn DeployFactor;

                    let dst = generate_dst!(item, self.settings, target_extract_dir_with_gen);

                    info!("template {} -> {}", item.name(), dst.display(),);
                    SecBuf::<Plain>::new(template.into_bytes()).deploy_to_fs(t, dst)
                })
                .for_each(|res| {
                    if let Err(e) = res {
                        error!("{}", e);
                    }
                });
        } else {
            info!("no template need to deploy. finished");
        }

        match std::fs::remove_file(symlink_dst) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            e @ Err(_) => e?,
            _ => debug!("old symlink removed"),
        }

        info!(
            "linking decrypted dir {} to {}",
            target_extract_dir_with_gen.display(),
            symlink_dst
        );
        std::os::unix::fs::symlink(target_extract_dir_with_gen, symlink_dst)
            .wrap_err_with(|| "create symlink error")
    }
}
