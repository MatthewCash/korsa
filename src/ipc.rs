use std::{env, io, os::unix::net::UnixDatagram, path::Path};

const DASHBOARD_SOCKET_NAME: &str = "ac-dashboard-refresh.sock";

pub fn notify_dashboard() {
    let Some(runtime_dir) = env::var_os("XDG_RUNTIME_DIR") else {
        return;
    };
    let path = Path::new(&runtime_dir).join(DASHBOARD_SOCKET_NAME);
    if let Err(error) = send_refresh(&path)
        && !matches!(
            error.kind(),
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
        )
    {
        log::warn!("could not notify race dashboard: {error}");
    }
}

fn send_refresh(path: &Path) -> io::Result<()> {
    UnixDatagram::unbound()?.send_to(b"refresh", path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, time::Duration};

    #[test]
    fn sends_dashboard_refresh_datagram() {
        let path = env::temp_dir().join(format!(
            "aclm-dashboard-refresh-test-{}.sock",
            std::process::id()
        ));
        fs::remove_file(&path).ok();
        let receiver = UnixDatagram::bind(&path).unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();

        send_refresh(&path).unwrap();

        let mut message = [0; 16];
        let length = receiver.recv(&mut message).unwrap();
        assert_eq!(&message[..length], b"refresh");
        fs::remove_file(path).unwrap();
    }
}
