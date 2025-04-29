use eyre::{Result, eyre};
use libc::{fchown, getgrnam, getpwnam};
use log::warn;
use std::{ffi::CString, fs::File, os::fd::AsRawFd};

pub fn set_owner_and_group(file: &File, owner: &str, group: &str) -> Result<()> {
    let fd = file.as_raw_fd();

    let user_uid = get_uid_from_username(owner).or_else(|_| {
        warn!("get uid of {owner} failed, fallback to uid 0");
        eyre::Ok(0)
    })?;
    let group_gid = get_gid_from_groupname(group).or_else(|_| {
        warn!("get gid of {group} failed, fallback to uid 0");
        eyre::Ok(0)
    })?;
    let result = unsafe { fchown(fd, user_uid, group_gid) };

    if result == -1 {
        eyre::bail!("set permission failed");
    }
    Ok(())
}

fn get_uid_from_username(username: &str) -> Result<u32> {
    let c_username = CString::new(username).map_err(|_| eyre!("Invalid username: {}", username))?;

    unsafe {
        let pw = getpwnam(c_username.as_ptr());
        if pw.is_null() {
            eyre::bail!("User '{}' not found", username);
        }
        Ok((*pw).pw_uid)
    }
}

fn get_gid_from_groupname(groupname: &str) -> Result<u32> {
    let c_groupname =
        CString::new(groupname).map_err(|_| eyre!("Invalid groupname: {}", groupname))?;

    unsafe {
        let gr = getgrnam(c_groupname.as_ptr());
        if gr.is_null() {
            eyre::bail!("Group '{}' not found", groupname);
        }
        Ok((*gr).gr_gid)
    }
}
