use std::{
    iter,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
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
                            m.iter()
                                .map(|(k, v)| {
                                    if sec_plain_map.contains_key(k) {
                                        return Ok((v, *k));
                                    }

                                    if let Ok(o) = ctx_agenc
                                        .inner_ref()
                                        .get(k)
                                        .wrap_err_with(|| eyre!("encrypted buf not found"))
                                        .and_then(|pl| pl.decrypt(key.as_ref()))
                                    {
                                        sec_plain_map.insert(*k, o);
                                    }
                                    Ok((v, *k))
                                })
                                .try_collect()
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

                let recip: Box<dyn Recipient + Send> = if let Ok(Ok(o)) =
                    String::from_str(h.recip())
                        .map(RecipString::from)
                        .map(RecipString::try_into)
                {
                    o
                } else {
                    res.lock()
                        .expect("doesn't matter now")
                        .push(Err(eyre!("parse host recipient fail")));
                    return;
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
                            res.lock()
                                .expect("doesn't matter now")
                                .push(Err(eyre!("create file error")));
                            return;
                        };

                        let dst_ctt_map_ref = dst_ctt_map.clone();

                        let buf = if let Some(o) = dst_ctt_map_ref
                            .get(inrepo_path)
                            .and_then(|s| sec_plain_map.get(*s))
                        {
                            o
                        } else {
                            res.lock().expect("must success").push(Err(eyre!(
                                "The secret store MAY contains stuff encrypted by other identity: {inrepo_path}"
                            )).wrap_err_with(||eyre!("Getting {:?} error", s)));
                            return;
                        };

                        let ctt = match buf.clone().encrypt(iter::once(recip.as_ref())) {
                            Ok(o) => o,
                            e @ Err(_) => {
                                res.lock()
                                    .expect("doesn't matter now")
                                    .push(e.map(|_| PathBuf::default()));
                                return;
                            }
                        };

                        if target_file.write_all(ctt.inner().as_bytes()).is_err() {
                            res.lock()
                                .expect("doesn't matter now")
                                .push(Err(eyre!("write cache file failed")))
                        };
                        res.lock()
                            .expect("thread work end")
                            .push(Ok(inrepo_path.path.clone()));
                    }
                });
            });
        });

        info!("finished");

        let last_res = res.lock().expect("never");

        last_res.iter().for_each(|i| {
            if let Err(e) = i {
                error!("{}", e);
            }
        });

        Ok(())
    }
}
