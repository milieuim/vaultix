use std::{
    iter,
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use age::{Identity, Recipient};
use dashmap::DashMap;
use log::{debug, error, info};
use nom::AsBytes;

use crate::{
    parser::recipient::RecipString,
    profile,
    util::{
        secbuf::{Decryptable, Plain, SecBuf},
        secmap::{InRepo, SecPathBuf},
    },
};

use super::{
    secbuf::AgeEnc,
    secmap::{RencCtx, RencInstance},
};

use eyre::{Context, ContextCompat, Result, eyre};

impl<'a> RencInstance<'a> {
    pub fn makeup(self, ctx_agenc: &RencCtx<'a, AgeEnc>, ident: Box<dyn Identity>) -> Result<()> {
        let key: Rc<dyn Identity> = Rc::from(ident);

        let material = &self.inner().into_read_only();

        info!("re-ecrypting...");

        use std::io::Write;

        let res: Arc<Mutex<Vec<eyre::Result<PathBuf>>>> = Arc::new(Mutex::new(Vec::new()));

        debug!(
            "total {} host(s) need to re-encrypt",
            material.keys().count()
        );
        std::thread::scope(|s| {
            material.iter().for_each(|(h, v)| {
                let key = key.clone();

                let sec_plain_map: Arc<DashMap<&profile::Secret, SecBuf<Plain>>> =
                    Arc::new(DashMap::new());

                let path_sec_map: Arc<DashMap<&SecPathBuf<InRepo>, &profile::Secret>> = Arc::new(
                    match material
                        .get(h)
                        .wrap_err_with(|| eyre!("never"))
                        .and_then(|m| {
                            let total = m.len();
                            m.iter()
                                .enumerate()
                                .map(|(i, (k, v))| {
                                    let kind = if sec_plain_map.contains_key(k) {
                                        "reuse"
                                    } else {
                                        "decrypt"
                                    };
                                    info!(
                                        "[{}/{}] {} {} for {}",
                                        i + 1,
                                        total,
                                        kind,
                                        k.file,
                                        h.id()
                                    );

                                    if sec_plain_map.contains_key(k) {
                                        return Ok((v, *k));
                                    }

                                    match ctx_agenc
                                        .inner_ref()
                                        .get(k)
                                        .wrap_err_with(|| eyre!("encrypted buf not found"))
                                        .and_then(|pl| pl.decrypt(key.as_ref()))
                                    {
                                        Ok(o) => {
                                            sec_plain_map.insert(*k, o);
                                        }
                                        Err(e) => {
                                            error!("decrypt secret {} failed: {e:#}", k.file);
                                        }
                                    }
                                    Ok((v, *k))
                                })
                                .collect::<Result<DashMap<_, _>, eyre::ErrReport>>()
                        }) {
                        Ok(o) => o,
                        e @ Err(_) => {
                            res.lock()
                                .expect("doesn't matter now")
                                .push(e.map(|_| PathBuf::default()));
                            return;
                        }
                    },
                );

                debug!("rencrypting for [{}]", h.id());
                let res = res.clone();
                let dst_ctt_map = path_sec_map.clone();
                let sec_plain_map = sec_plain_map.clone();

                let recip: Box<dyn Recipient + Send> =
                    match RecipString::from(h.recip().to_owned()).try_into() {
                        Ok(recipient) => recipient,
                        Err(error) => {
                            res.lock()
                                .expect("doesn't matter now")
                                .push(Err(error.wrap_err(format!(
                                    "Failed to parse host recipient for `{}`",
                                    h.id()
                                ))));
                            return;
                        }
                    };

                s.spawn(move || {
                    for (_, inrepo_path) in v.iter() {
                        if let Err(e) = inrepo_path
                            .path
                            .parent()
                            .wrap_err_with(|| {
                                eyre!("cache file path has no parent, is this possible?")
                            })
                            .and_then(|i| {
                                std::fs::create_dir_all(i)
                                    .wrap_err_with(|| eyre!("create host cache dir in repo failed"))
                            })
                        {
                            error!("{e}");
                            res.lock().expect("doesn't matter now").push(Err(e));
                            return;
                        };
                        let mut target_file = if let Ok(o) = std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(inrepo_path.path.clone())
                        {
                            o
                        } else {
                            let e = eyre!("create cache file error: {inrepo_path}");
                            error!("{e}");
                            res.lock()
                                .expect("doesn't matter now")
                                .push(Err(e));
                            return;
                        };

                        let dst_ctt_map_ref = dst_ctt_map.clone();

                        let buf = if let Some(o) = dst_ctt_map_ref
                            .get(inrepo_path)
                            .and_then(|sp| sec_plain_map.get(*sp))
                        {
                            o
                        } else {
                            let e = eyre!(
                                "secret not decrypted for {inrepo_path}: it may be encrypted with a different identity"
                            );
                            error!("{e}");
                            res.lock().expect("must success").push(Err(e));
                            return;
                        };

                        let ctt = match buf.clone().encrypt(iter::once(recip.as_ref())) {
                            Ok(o) => o,
                            Err(e) => {
                                error!("encrypt failed for {inrepo_path}: {e}");
                                res.lock()
                                    .expect("doesn't matter now")
                                    .push(Err(e));
                                return;
                            }
                        };

                        if let Err(e) = target_file.write_all(ctt.inner().as_bytes()) {
                            let e = eyre!("write cache file failed: {e}");
                            error!("{e}");
                            res.lock()
                                .expect("doesn't matter now")
                                .push(Err(e));
                        } else {
                            res.lock()
                                .expect("thread work end")
                                .push(Ok(inrepo_path.path.clone()));
                        }
                    }
                });
            });
        });

        info!("finished");

        let last_res = res.lock().expect("never");

        let errs: Vec<_> = last_res.iter().filter_map(|i| i.as_ref().err()).collect();
        for e in &errs {
            error!("{e:#}");
        }

        if errs.is_empty() {
            Ok(())
        } else {
            Err(eyre!("{} host(s) failed to re-encrypt", errs.len()))
        }
    }
}
