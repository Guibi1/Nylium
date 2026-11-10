use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::Path,
    process::Command,
    time::{Duration, Instant},
};

use nylium_adapter::{FieldValue, Lifecycle, Section, ServerAdapter};
use tempfile::tempdir;
use toml_edit::DocumentMut;

use crate::PumpkinAdapter;

const CHILD_MARKER: &str = "NYLIUM_PUMPKIN_LIFECYCLE_CHILD";
const GAME_RULE_ID: &str = "minecraft:overworld#minecraft:random_tick_speed";

#[test]
fn real_server_lifecycle_survives_restart() {
    if std::env::var_os(CHILD_MARKER).is_some() {
        run_real_server_lifecycle();
        return;
    }

    let dir = tempdir().unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "lifecycle_tests::real_server_lifecycle_survives_restart",
            "--nocapture",
        ])
        .env(CHILD_MARKER, "1")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "real Pumpkin lifecycle failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_real_server_lifecycle() {
    let config_path = Path::new("pumpkin.toml");
    let adapter = PumpkinAdapter::new(".").unwrap();
    adapter.install_panic_handler();
    let first_address = free_local_address();
    edit_config(config_path, first_address, "Initial lifecycle test MOTD");

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        adapter.start().await.unwrap();
        adapter.start().await.unwrap();
        wait_for(|| {
            let snapshot = adapter.snapshot();
            snapshot.lifecycle == Lifecycle::Running && snapshot.connected
        })
        .await;
        assert_tcp_status(first_address);

        let config_before_invalid_update = std::fs::read(config_path).unwrap();
        assert!(
            adapter
                .set_field(
                    Section::Settings,
                    "advanced.networking.java.view_distance".into(),
                    FieldValue::Number(1),
                )
                .await
                .is_err()
        );
        assert_eq!(
            std::fs::read(config_path).unwrap(),
            config_before_invalid_update
        );
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert_eq!(adapter.snapshot().lifecycle, Lifecycle::Running);
        assert!(adapter.snapshot().connected);

        adapter
            .set_field(
                Section::GameRules,
                GAME_RULE_ID.into(),
                FieldValue::Text("17".into()),
            )
            .await
            .unwrap();
        wait_for(|| {
            adapter.snapshot().game_rules.iter().any(|field| {
                field.id == GAME_RULE_ID && field.value == FieldValue::Text("17".into())
            })
        })
        .await;

        let second_address = free_local_address();
        edit_config(
            config_path,
            second_address,
            "Externally updated lifecycle test MOTD",
        );
        wait_for(|| {
            let snapshot = adapter.snapshot();
            snapshot.restart_required
                && snapshot.settings.iter().any(|field| {
                    field.id == "advanced.networking.java.motd"
                        && field.value
                            == FieldValue::Text("Externally updated lifecycle test MOTD".into())
                })
        })
        .await;

        adapter.stop().await.unwrap();
        adapter.stop().await.unwrap();
        assert_eq!(adapter.snapshot().lifecycle, Lifecycle::Stopped);

        adapter.start().await.unwrap();
        adapter.start().await.unwrap();
        wait_for(|| {
            let snapshot = adapter.snapshot();
            snapshot.lifecycle == Lifecycle::Running
                && snapshot.connected
                && snapshot.game_rules.iter().any(|field| {
                    field.id == GAME_RULE_ID && field.value == FieldValue::Text("17".into())
                })
        })
        .await;
        assert_tcp_status(second_address);
        assert!(adapter.snapshot().settings.iter().any(|field| {
            field.id == "advanced.networking.java.motd"
                && field.value == FieldValue::Text("Externally updated lifecycle test MOTD".into())
        }));
        adapter.stop().await.unwrap();
        assert_eq!(adapter.snapshot().lifecycle, Lifecycle::Stopped);
    });
}

fn edit_config(path: &Path, address: SocketAddr, motd: &str) {
    let source = std::fs::read_to_string(path).unwrap();
    let mut doc = source.parse::<DocumentMut>().unwrap();
    doc["networking"]["java"]["address"] = toml_edit::value(address.to_string());
    doc["networking"]["java"]["online_mode"] = toml_edit::value(false);
    doc["networking"]["java"]["authentication"]["enabled"] = toml_edit::value(false);
    doc["networking"]["java"]["motd"] = toml_edit::value(motd);
    doc["networking"]["bedrock"]["enabled"] = toml_edit::value(false);
    doc["networking"]["bedrock"]["nethernet"]["enabled"] = toml_edit::value(false);
    doc["networking"]["lan_broadcast"]["enabled"] = toml_edit::value(false);
    doc["networking"]["query"]["enabled"] = toml_edit::value(false);
    doc["networking"]["management"]["enabled"] = toml_edit::value(false);
    doc["update_checker"]["enabled"] = toml_edit::value(false);
    doc["telemetry"]["enabled"] = toml_edit::value(false);
    doc["use_favicon"] = toml_edit::value(false);
    std::fs::write(path, doc.to_string()).unwrap();
}

fn free_local_address() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap()
}

fn assert_tcp_status(address: SocketAddr) {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Ok(mut stream) = TcpStream::connect_timeout(&address, Duration::from_millis(200)) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            if request_status(&mut stream).is_ok_and(|status| {
                status.contains("\"version\"") && status.contains("\"players\"")
            }) {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "Pumpkin did not return Java status on {address}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn request_status(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0);
    write_varint(&mut handshake, 0);
    write_string(&mut handshake, &stream.peer_addr()?.ip().to_string());
    handshake.extend_from_slice(&stream.peer_addr()?.port().to_be_bytes());
    write_varint(&mut handshake, 1);
    let mut packet = Vec::new();
    write_varint(&mut packet, handshake.len() as i32);
    packet.extend_from_slice(&handshake);
    packet.extend_from_slice(&[1, 0]); // Status request packet.
    stream.write_all(&packet)?;

    let frame_len = read_varint(stream)?;
    if frame_len < 1 || read_varint(stream)? != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid Java status packet",
        ));
    }
    let json_len = read_varint(stream)?;
    if json_len < 0 || json_len > frame_len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid Java status payload length",
        ));
    }
    let mut json = vec![0; json_len as usize];
    stream.read_exact(&mut json)?;
    String::from_utf8(json)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn write_string(packet: &mut Vec<u8>, value: &str) {
    write_varint(packet, value.len() as i32);
    packet.extend_from_slice(value.as_bytes());
}

fn write_varint(packet: &mut Vec<u8>, mut value: i32) {
    loop {
        if value & !0x7f == 0 {
            packet.push(value as u8);
            return;
        }
        packet.push((value & 0x7f | 0x80) as u8);
        value = ((value as u32) >> 7) as i32;
    }
}

fn read_varint(stream: &mut TcpStream) -> std::io::Result<i32> {
    let mut value = 0_u32;
    for shift in (0..35).step_by(7) {
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        value |= u32::from(byte[0] & 0x7f) << shift;
        if byte[0] & 0x80 == 0 {
            return Ok(value as i32);
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "invalid VarInt",
    ))
}

async fn wait_for(check: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !check() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for Pumpkin state"
        );
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
