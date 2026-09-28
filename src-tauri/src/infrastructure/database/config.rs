//! `my.ini` generation (phase-a2 A2-5, P2-48): a pure function from paths + port + bind mode to
//! the exact config text written to `database\my.ini` before every start. `mariadbd` is always
//! launched as `mariadbd.exe --defaults-file=<my.ini>` — nothing is passed on the command line
//! that isn't also in this file, so the config is fully inspectable/diagnosable from one place.

use super::paths::ServerPaths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindMode {
    /// Only `127.0.0.1` — the default; no LAN sharing.
    Loopback,
    /// `0.0.0.0` — A3's LAN sharing, reachable from other terminals on the network.
    Lan,
}

pub struct ServerConfig<'a> {
    pub paths: &'a ServerPaths,
    pub port: u16,
    pub bind: BindMode,
}

/// Pure: same inputs always produce the same text, so this is unit-tested without touching disk
/// or spawning anything.
pub fn render_my_ini(cfg: &ServerConfig) -> String {
    let ini = ServerPaths::to_ini_path;
    let bind_address = match cfg.bind {
        BindMode::Loopback => "127.0.0.1",
        BindMode::Lan => "0.0.0.0",
    };

    format!(
        "[mariadbd]\n\
basedir={basedir}\n\
datadir={datadir}\n\
plugin-dir={plugin_dir}\n\
lc-messages-dir={lc_messages_dir}\n\
tmpdir={tmpdir}\n\
pid-file={pid_file}\n\
log-error={log_error}\n\
port={port}\n\
bind-address={bind_address}\n\
skip-name-resolve\n\
max_connections=151\n\
innodb_buffer_pool_size=256M\n\
innodb_log_file_size=64M\n\
innodb_flush_log_at_trx_commit=1\n\
innodb_file_per_table=1\n\
character-set-server=utf8mb4\n\
collation-server=utf8mb4_unicode_ci\n\
default_storage_engine=InnoDB\n\
sql_mode=STRICT_ALL_TABLES,NO_ZERO_IN_DATE,NO_ZERO_DATE,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION\n\
skip-log-bin\n\
skip-networking=0\n",
        basedir = ini(&cfg.paths.server_bin_current()),
        datadir = ini(&cfg.paths.data()),
        plugin_dir = ini(&cfg.paths.server_bin_current().join("lib").join("plugin")),
        lc_messages_dir = ini(&cfg.paths.server_bin_current().join("share")),
        tmpdir = ini(&cfg.paths.tmp()),
        pid_file = ini(&cfg.paths.pid_file()),
        log_error = ini(&cfg.paths.error_log()),
        port = cfg.port,
        bind_address = bind_address,
    )
}

/// Writes `my.ini` atomically (temp file + rename), same mechanism as `state_file.rs`.
pub fn write_my_ini(cfg: &ServerConfig) -> std::io::Result<()> {
    let text = render_my_ini(cfg);
    let path = cfg.paths.my_ini();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("ini.tmp");
    std::fs::write(&tmp_path, text)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_output_has_no_backslash_in_any_path() {
        let paths = ServerPaths::at(r"C:\ProgramData\com.abdallah.accounting-app\database");
        let cfg = ServerConfig { paths: &paths, port: 3406, bind: BindMode::Loopback };
        let ini = render_my_ini(&cfg);
        assert!(!ini.contains('\\'), "my.ini must never contain a backslash: {ini}");
        assert!(ini.contains("port=3406"));
        assert!(ini.contains("bind-address=127.0.0.1"));
        assert!(ini.contains("[mariadbd]"));
    }

    #[test]
    fn lan_bind_mode_binds_all_interfaces() {
        let paths = ServerPaths::at(r"C:\root");
        let cfg = ServerConfig { paths: &paths, port: 3406, bind: BindMode::Lan };
        let ini = render_my_ini(&cfg);
        assert!(ini.contains("bind-address=0.0.0.0"));
    }
}
