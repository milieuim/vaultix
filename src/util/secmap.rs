use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::{
    cmd::renc::CompleteProfile,
    profile::{self, Secret},
    util::secbuf::AgeEnc,
};
use age::Identity;
use dashmap::{DashMap, Map};
use eyre::{Context, bail};
use eyre::{Result, eyre};
use log::debug;
use std::marker::PhantomData;

use super::secbuf::{Decryptable, HostEnc, SecBuf};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SecPath<P: AsRef<Path>, T> {
    pub path: P,
    _marker: PhantomData<T>,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct InStore;
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct InRepo;

pub type SecPathBuf<A> = SecPath<PathBuf, A>;

pub trait GetSec {
    fn read_buffer(&self) -> Result<Vec<u8>>;
    fn open_file(&self) -> Result<File>;
}

impl<P, T> SecPath<P, T>
where
    P: AsRef<Path>,
{
    pub fn new(path: P) -> Self {
        SecPath {
            path,
            _marker: PhantomData,
        }
    }
}

impl<P, T> GetSec for SecPath<P, T>
where
    P: AsRef<Path>,
{
    fn open_file(&self) -> Result<File> {
        File::open(&self.path).wrap_err_with(|| eyre!("open secret file error"))
    }

    fn read_buffer(&self) -> Result<Vec<u8>> {
        let mut f = self.open_file()?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .wrap_err_with(|| eyre!("read secret file error"))?;
        Ok(buffer)
    }
}
// identifier, recip
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HostInfo<'a>(&'a str, &'a str);
impl<'a> HostInfo<'a> {
    pub fn id(&self) -> &'a str {
        self.0
    }
    pub fn recip(&self) -> &str {
        self.1
    }
}
#[derive(Debug, Clone)]
pub struct RencBuilder<'a>(
    std::collections::HashMap<(&'a Secret, SecPathBuf<InStore>), Vec<HostInfo<'a>>>,
);

#[derive(Debug, Clone)]
pub struct RencData<'a, W>(std::collections::HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>>);

#[derive(Debug, Clone)]
pub struct RencInstance<'a>(dashmap::DashMap<HostInfo<'a>, Vec<(&'a Secret, SecPathBuf<InRepo>)>>);

impl<'a> RencInstance<'a> {
    pub fn inner(self) -> dashmap::DashMap<HostInfo<'a>, Vec<(&'a Secret, SecPathBuf<InRepo>)>> {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct RencCtx<'a, B>(DashMap<&'a Secret, SecBuf<B>>);

impl<'a, B> RencCtx<'a, B> {
    pub fn inner_ref(&self) -> &DashMap<&'a Secret, SecBuf<B>> {
        &self.0
    }
}

impl<'a> RencCtx<'a, AgeEnc> {
    pub fn create(material: &'a CompleteProfile) -> Result<Self> {
        let c: DashMap<&Secret, Result<SecBuf<AgeEnc>>> = material
            .inner_ref()
            .iter()
            .flat_map(|x| x.secrets.values())
            .map(|i| {
                (
                    i,
                    PathBuf::from(i.file.clone())
                        .canonicalize()
                        .wrap_err_with(|| eyre!("secret not found: {}", i.file))
                        .and_then(|i| {
                            SecPathBuf::<InStore>::from(&i)
                                .read_buffer()
                                .map(SecBuf::new)
                        }),
                )
            })
            .collect();

        for ref r in c.iter() {
            if r.is_err() {
                bail!("{}", r.as_ref().unwrap_err())
            }
        }

        Ok(Self(
            c.into_iter()
                .map(|(k, v)| (k, v.expect("handled")))
                .collect(),
        ))
    }
}

impl<'a, W> RencData<'a, W> {
    pub fn inner(self) -> HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>> {
        self.0
    }
    fn inner_ref(&self) -> &HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>> {
        &self.0
    }
    fn inner_ref_mut(&mut self) -> &mut HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>> {
        &mut self.0
    }
    fn have(&self, p: &PathBuf) -> bool {
        for ip in self.inner_ref().values() {
            if &ip.path == p {
                return true;
            }
        }
        false
    }
}
impl<'a> RencBuilder<'a> {
    pub fn create(profiles: &'a CompleteProfile) -> Self {
        // hostinfo - <S-P(S)>
        let a: HashMap<HostInfo, HashMap<&Secret, SecPathBuf<InStore>>> = profiles
            .inner_ref()
            .iter()
            .map(|x| {
                let host_info = HostInfo(x.host_identifier(), x.host_pubkey());
                let s_ps: HashMap<&Secret, SecPathBuf<InStore>> = x
                    .secrets
                    .values()
                    .map(|n| (n, SecPathBuf::<InStore>::from(n)))
                    .collect();
                (host_info, s_ps)
            })
            .collect();

        // (S, P(S)) - [host]
        let b: HashMap<(&Secret, SecPathBuf<InStore>), Vec<HostInfo>> =
            a.iter().fold(HashMap::new(), |mut acc, (x, y)| {
                y.iter().for_each(|(n, i)| {
                    acc.entry((*n, i.clone()))
                        .and_modify(|i| i.push(x.clone()))
                        .or_insert_with(|| vec![x.clone()]);
                });
                acc
            });
        Self(b)
    }

    pub fn build_inrepo(
        self,
        ctx: &RencCtx<'a, AgeEnc>,
        cache_dir: PathBuf,
    ) -> RencData<'a, InRepo> {
        RencData::<'_, InRepo>(self.0.iter().fold(HashMap::new(), |mut acc, ((x, _), z)| {
            z.iter().for_each(|h| {
                let hash = ctx.0.get(x).expect("never").hash_with(h.recip());
                let in_repo = {
                    let mut p: PathBuf = cache_dir.clone();
                    p.push(h.0);
                    p.push(hash.to_string());
                    SecPathBuf::<InRepo>::new(p)
                };
                acc.insert((*x, h.clone()), in_repo);
            });
            acc
        }))
    }

    pub fn build_instore(self) -> RencData<'a, InStore> {
        RencData::<'_, InStore>(self.0.iter().fold(HashMap::new(), |mut acc, ((x, y), z)| {
            z.iter().for_each(|h| {
                acc.insert((*x, h.clone()), y.clone());
            });
            acc
        }))
    }
}
impl<'a> RencData<'a, InStore> {
    /// return self but processed the path to produce in-store cache/host/[hash] map
    pub fn renced_stored(self, ctx: &RencCtx<'a, AgeEnc>, host_cache_stored: PathBuf) -> Self {
        self.inner_ref()
            .iter()
            .map(|((x, y), _)| {
                let mut dir = host_cache_stored.clone();
                let sec_hash = ctx
                    .inner_ref()
                    .get(x)
                    .expect("must have")
                    .hash_with(y.recip())
                    .to_string();

                dir.push(sec_hash);

                ((*x, y.clone()), SecPath::new(dir))
            })
            .collect::<HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<InStore>>>()
            .into()
    }
    /// read secret file
    pub fn bake_decrypted(
        self,
        idents: Vec<Box<dyn Identity>>,
    ) -> Result<HashMap<&'a Secret, Vec<u8>>> {
        let mut verified_key_idx = None;
        self.inner()
            .into_iter()
            .map(|(k, v)| {
                v.read_buffer()
                    .and_then(|b| {
                        if let Some(thekey_idx) = verified_key_idx {
                            return SecBuf::<HostEnc>::from(b)
                                .decrypt((&idents[thekey_idx] as &Box<dyn Identity>).as_ref());
                        }

                        // iterate identitys to find the correct key
                        for (idx, k) in idents.iter().enumerate() {
                            let decrypt_res =
                                SecBuf::<HostEnc>::from(b.clone()).decrypt(k.as_ref());
                            if decrypt_res.is_ok() {
                                verified_key_idx = Some(idx);
                                return decrypt_res;
                            }
                        }
                        Err(eyre!("no key is able to decrypt the secret"))
                    })
                    .map(|i| (k.0, i.inner()))
            })
            .try_collect::<HashMap<&'a Secret, Vec<u8>>>()
    }
}

impl<'a> RencData<'a, InRepo> {
    pub fn clean_outdated(&self, cache_dir: PathBuf) -> Result<()> {
        self.inner_ref().keys().map(|(_, v)| v).try_for_each(|h| {
            let host_cache_dir = {
                let mut c = cache_dir.clone();
                c.push(h.0);
                c
            };
            let dir = std::fs::read_dir(host_cache_dir);

            if let Err(ref e) = dir
                && e.kind() == io::ErrorKind::NotFound
            {
                return Ok(());
            }

            let dir = dir?;

            let tobe_clean = dir.filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_file() && !self.have(&path) {
                    Some(path)
                } else {
                    None
                }
            });

            for p in tobe_clean {
                debug!("cleaning old: {}", p.display());
                std::fs::remove_file(p).with_context(|| eyre!("cleaning old renc file error"))?;
            }
            Ok(())
        })
    }

    /// retain non exist path
    pub fn retain_noexist(&mut self) {
        // TODO: check if all cache added to git?
        self.inner_ref_mut().retain(|_, v| !v.path.exists())
    }

    pub fn build_instance(&self) -> RencInstance<'a> {
        RencInstance(
            self.inner_ref()
                .iter()
                .fold(DashMap::new(), |acc, ((x, y), z)| {
                    if !acc.contains_key(y) {
                        acc.insert(y.clone(), vec![(*x, z.clone())]);
                    }
                    acc._entry(y.clone()).and_modify(|i| {
                        i.push((x, z.clone()));
                    });

                    acc
                }),
        )
    }
}

impl<'a> FromIterator<((&'a profile::Secret, HostInfo<'a>), SecPathBuf<InRepo>)>
    for RencData<'a, InRepo>
{
    fn from_iter<
        T: IntoIterator<Item = ((&'a profile::Secret, HostInfo<'a>), SecPathBuf<InRepo>)>,
    >(
        iter: T,
    ) -> Self {
        let mut m = HashMap::new();
        for i in iter.into_iter() {
            m.insert(i.0, i.1);
        }
        Self(m)
    }
}
impl<'a> From<RencBuilder<'a>> for RencData<'a, InStore> {
    fn from(value: RencBuilder<'a>) -> Self {
        value.build_instore()
    }
}
impl<P: AsRef<Path>, T> fmt::Display for SecPath<P, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.as_ref().display())
    }
}
impl<'a> From<&'a profile::Secret> for SecPathBuf<InStore> {
    fn from(value: &'a profile::Secret) -> Self {
        Self {
            path: value.file.clone().into(),
            _marker: PhantomData,
        }
    }
}
impl<'a> From<&'a PathBuf> for SecPathBuf<InStore> {
    fn from(value: &'a PathBuf) -> Self {
        Self {
            path: value.to_owned(),
            _marker: PhantomData,
        }
    }
}
impl<'a> From<HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<InStore>>> for RencData<'a, InStore> {
    fn from(value: HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<InStore>>) -> Self {
        Self(value)
    }
}
impl<'a> FromIterator<(&'a profile::Secret, SecBuf<AgeEnc>)> for RencCtx<'a, AgeEnc> {
    fn from_iter<T: IntoIterator<Item = (&'a profile::Secret, SecBuf<AgeEnc>)>>(iter: T) -> Self {
        let m = DashMap::new();
        for i in iter.into_iter() {
            m.insert(i.0, i.1);
        }
        Self(m)
    }
}
