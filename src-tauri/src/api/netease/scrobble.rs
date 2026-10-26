use std::collections::HashMap;

use rand::{Rng, RngCore};
use reqwest::{multipart, Client};
use rsa::BigUint;
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};

const UPLOAD_URL: &str =
    "https://clientlog3.music.163.com/api/clientlog/encrypt/upload?multiupload=true";
const FIELD_SEP: char = '\u{1}';
const RSA_MODULUS: &[u8] = b"FD90BD466FF9BC8A3FEC2FBCF263B90D5C564879FA5D7AAB89B31C1D5CB4139D";
const SIGMA: [u32; 4] = [0x61707865, 0x3320646e, 0x79622d32, 0x6b206574];

#[derive(Debug, Clone)]
pub struct ScrobbleSong {
    pub id: u64,
    pub total_time_ms: u64,
    pub played_ms: u64,
}

pub async fn report(
    client: &Client,
    cookies: &HashMap<String, String>,
    song: &ScrobbleSong,
) -> AppResult<()> {
    if cookies.get("MUSIC_U").is_none_or(|value| value.is_empty()) {
        return Err(AppError::Api("Netease not logged in".into()));
    }

    let mut cookies = cookies.clone();
    if cookies.get("WNMCID").is_none_or(|value| value.is_empty()) {
        let mut rng = rand::thread_rng();
        let prefix: String = (0..6)
            .map(|_| char::from(b'a' + rng.gen_range(0..26)))
            .collect();
        cookies.insert(
            "WNMCID".into(),
            format!("{prefix}.{}.01.0", chrono::Utc::now().timestamp_millis()),
        );
    }
    let now_ms = chrono::Utc::now().timestamp_millis();
    let now_sec = now_ms / 1000;
    let total_time_ms = song.total_time_ms.max(song.played_ms);
    let total_sec = total_time_ms / 1000;
    let played_sec = song.played_ms.min(total_time_ms) / 1000;
    let source_id = song.id.to_string();
    let meta = build_meta(&cookies).to_string();
    let cookie_header = build_cookie_header(&cookies);
    let app_version = cookie(&cookies, "appver", "3.1.35");
    let channel = cookie(&cookies, "channel", "netease");
    let plv_add_refer = format!(
        "[F:63][{now_ms}#933#{app_version}#205293#c9156c3][e][2][23][cell_pc_songlist_song:2|page_pc_songlist_songflow|page_mine_like_music][{}:song:x:x|:::|{}:list::]",
        song.id, song.id
    );
    let pld_add_refer = format!(
        "[F:63][{now_ms}#616#{app_version}#205293#c9156c3][e][2][92][btn_pc_cover_play|cell_pc_songlist_song:6|page_pc_songlist_songflow|page_mine_like_music][:::|{}:song:x:x|:::|{}:list::]",
        song.id, song.id
    );

    let plv = json!({
        "mode": "circulation", "download": 0, "alg": "", "status": "front",
        "id": source_id, "bitrate": 320, "type": "song", "is_listentogether": 0,
        "source": "list", "is_heart": 0, "resource_ratio": "", "resource_time": total_sec,
        "musiceffect_id": "", "app_mode": 2, "bitrate_level": "exhigh",
        "_addrefer": plv_add_refer,
        "_multirefers": ["[F:26][s][18][_ai]", "[F:26][s][12][_ai]", "[F:26][s][5][_ai]", "[F:26][s][0][_ai]"],
        "vipType": cookie(&cookies, "vipType", ""), "fee": 1, "file": 4,
        "rightSource": 0, "sourceId": source_id, "sourcetype": "track",
        "libra_abt": "", "channel": channel,
        "curStartChannel": ""
    });
    let pld = json!({
        "mode": "circulation", "download": 0, "alg": "", "status": "front",
        "id": source_id, "time": played_sec, "type": "song", "is_listentogether": 0,
        "source": "list", "is_heart": 0, "realtime": played_sec,
        "resource_ratio": "", "resource_time": total_sec, "musiceffect_id": "1001",
        "app_mode": 1, "lyriceffect": "default", "displayMode": "classic",
        "_addrefer": pld_add_refer,
        "_multirefers": ["[F:26][s][87][_ai]", "[F:26][s][81][_ai]", "[F:26][s][75][_ai]", "[F:26][s][69][_ai]", "[F:26][s][63][_ai]"],
        "bitrate": 320, "bitrate_level": "exhigh", "vipType": cookie(&cookies, "vipType", ""),
        "fee": 8, "file": 4, "rightSource": 0, "sourceId": source_id,
        "sourcetype": "track", "end": "interrupt", "libra_abt": "",
        "channel": channel, "curStartChannel": ""
    });

    for (sequence, (action, data)) in [("_plv", plv), ("_pld", pld)].into_iter().enumerate() {
        let body = format!("{now_sec}{FIELD_SEP}{action}{FIELD_SEP}{data}");
        let payload = encrypt_ncbl(&meta, &body)?;
        upload(client, &cookie_header, app_version, payload, sequence).await?;
    }
    Ok(())
}

fn cookie<'a>(cookies: &'a HashMap<String, String>, name: &str, fallback: &'a str) -> &'a str {
    cookies.get(name).map(String::as_str).unwrap_or(fallback)
}

fn build_meta(cookies: &HashMap<String, String>) -> Value {
    json!({
        "JSESSIONID-WYYY": cookie(cookies, "JSESSIONID-WYYY", ""),
        "MUSIC_U": cookie(cookies, "MUSIC_U", ""),
        "NMTID": cookie(cookies, "NMTID", ""),
        "WEVNSM": cookie(cookies, "WEVNSM", "1.0.0"),
        "WNMCID": cookie(cookies, "WNMCID", ""),
        "__csrf": cookie(cookies, "__csrf", ""),
        "_iuqxldmzr_": "33", "_ntes_nnid": cookie(cookies, "_ntes_nnid", ","),
        "_ntes_nuid": cookie(cookies, "_ntes_nuid", ""),
        "appver": format!("{}.205293", cookie(cookies, "appver", "3.1.35")),
        "channel": cookie(cookies, "channel", "netease"),
        "clientSign": cookie(cookies, "clientSign", ""),
        "deviceId": cookie(cookies, "deviceId", cookie(cookies, "sDeviceId", "")),
        "mode": cookie(cookies, "mode", ""), "ntes_kaola_ad": "1",
        "os": cookie(cookies, "os", "pc"),
        "osver": cookie(cookies, "osver", "Microsoft-Windows-11-Professional-build-26100-64bit")
    })
}

fn build_cookie_header(cookies: &HashMap<String, String>) -> String {
    let mut values = cookies
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>();
    values.sort();
    values.join("; ")
}

async fn upload(
    client: &Client,
    cookie_header: &str,
    app_version: &str,
    payload: Vec<u8>,
    sequence: usize,
) -> AppResult<()> {
    let file_name = {
        let mut rng = rand::thread_rng();
        format!(
            "op_{}_{}_{}",
            rng.gen_range(10000..100000),
            sequence,
            rng.gen::<u32>()
        )
    };
    let form = multipart::Form::new().part(
        "file",
        multipart::Part::bytes(payload)
            .file_name(file_name.clone())
            .mime_str("multipart/form-data")?,
    );
    let response = client
        .post(UPLOAD_URL)
        .header("Referer", "https://music.163.com/di")
        .header("Cookie", cookie_header)
        .header(
            "User-Agent",
            format!("Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 NeteaseMusicDesktop/{app_version}"),
        )
        .multipart(form)
        .send()
        .await?;
    let body: Value = response.json().await?;
    let matched = body["data"]["successfiles"]
        .as_array()
        .is_some_and(|files| files.iter().any(|value| value.as_str() == Some(&file_name)));
    if body["code"].as_i64() == Some(200) && matched {
        Ok(())
    } else {
        Err(AppError::Api(format!(
            "Netease scrobble upload failed: {body}"
        )))
    }
}

fn encrypt_ncbl(meta: &str, body: &str) -> AppResult<Vec<u8>> {
    let mut rng = rand::thread_rng();
    let mut key_a = [0_u8; 32];
    rng.fill_bytes(&mut key_a);
    if key_a[0] >= 0xa3 {
        key_a[0] = 0xa2;
    }
    let key_b = rsa_wrap(&key_a)?;
    let mut uuid = [0_u8; 16];
    rng.fill_bytes(&mut uuid);
    uuid[6] = (uuid[6] & 0x0f) | 0x40;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;
    let nonce: [u8; 12] = uuid[..12].try_into().expect("fixed nonce length");
    let counter = u32::from_le_bytes(uuid[12..16].try_into().expect("fixed counter length")) >> 2;
    let base_sequence = rng.gen::<u16>() as u32;

    let meta_cipher = chacha20(&key_b, counter, &nonce, meta.as_bytes());
    let compressed = zstd::stream::encode_all(body.as_bytes(), 0)
        .map_err(|error| AppError::Other(format!("Zstandard encode: {error}")))?;
    let mut frames = Vec::new();
    let mut frame_count = 0_u32;
    for (index, chunk) in compressed.chunks(0x8000).enumerate() {
        let cipher = chacha20(&key_a, counter, &nonce, chunk);
        frames.extend_from_slice(&(cipher.len() as u16).to_le_bytes());
        frames.extend_from_slice(&(base_sequence + index as u32).to_le_bytes());
        frames.extend_from_slice(&cipher);
        frame_count += 1;
    }

    let header_len = 70 + 4 + meta_cipher.len();
    let mut output = Vec::with_capacity(header_len + frames.len());
    output.extend_from_slice(b"NCBL");
    output.extend_from_slice(&3_u32.to_le_bytes());
    output.extend_from_slice(&(header_len as u16).to_le_bytes());
    output.extend_from_slice(&uuid);
    output.extend_from_slice(&key_b);
    output.extend_from_slice(&base_sequence.to_le_bytes());
    output.extend_from_slice(&(base_sequence + frame_count.saturating_sub(1)).to_le_bytes());
    output.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    output.extend_from_slice(&0x4343_u16.to_le_bytes());
    output.extend_from_slice(&(meta_cipher.len() as u16).to_le_bytes());
    output.extend_from_slice(&meta_cipher);
    output.extend_from_slice(&frames);
    Ok(output)
}

fn rsa_wrap(key: &[u8; 32]) -> AppResult<[u8; 32]> {
    let modulus = BigUint::parse_bytes(RSA_MODULUS, 16)
        .ok_or_else(|| AppError::Other("Invalid NCBL RSA modulus".into()))?;
    let wrapped = BigUint::from_bytes_be(key)
        .modpow(&BigUint::from(65537_u32), &modulus)
        .to_bytes_be();
    let mut output = [0_u8; 32];
    let source = if wrapped.len() > 32 {
        &wrapped[wrapped.len() - 32..]
    } else {
        &wrapped
    };
    output[32 - source.len()..].copy_from_slice(source);
    Ok(output)
}

fn chacha20(key: &[u8; 32], counter: u32, nonce: &[u8; 12], data: &[u8]) -> Vec<u8> {
    let mut output = vec![0_u8; data.len()];
    for (block_index, chunk) in data.chunks(64).enumerate() {
        let stream = chacha_block(key, counter.wrapping_add(block_index as u32), nonce);
        let offset = block_index * 64;
        for (index, byte) in chunk.iter().enumerate() {
            output[offset + index] = byte ^ stream[index];
        }
    }
    output
}

fn chacha_block(key: &[u8; 32], counter: u32, nonce: &[u8; 12]) -> [u8; 64] {
    let mut state = [0_u32; 16];
    state[..4].copy_from_slice(&SIGMA);
    for index in 0..8 {
        state[4 + index] = u32::from_le_bytes(key[index * 4..index * 4 + 4].try_into().unwrap());
    }
    state[12] = counter;
    for index in 0..3 {
        state[13 + index] = u32::from_le_bytes(nonce[index * 4..index * 4 + 4].try_into().unwrap());
    }
    let mut work = state;
    for _ in 0..10 {
        quarter_round(&mut work, 0, 4, 8, 12);
        quarter_round(&mut work, 1, 5, 9, 13);
        quarter_round(&mut work, 2, 6, 10, 14);
        quarter_round(&mut work, 3, 7, 11, 15);
        quarter_round(&mut work, 0, 5, 10, 15);
        quarter_round(&mut work, 1, 6, 11, 12);
        quarter_round(&mut work, 2, 7, 8, 13);
        quarter_round(&mut work, 3, 4, 9, 14);
    }
    let mut output = [0_u8; 64];
    for index in 0..16 {
        output[index * 4..index * 4 + 4]
            .copy_from_slice(&work[index].wrapping_add(state[index]).to_le_bytes());
    }
    output
}

fn quarter_round(state: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize) {
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(16);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(12);
    state[a] = state[a].wrapping_add(state[b]);
    state[d] ^= state[a];
    state[d] = state[d].rotate_left(8);
    state[c] = state[c].wrapping_add(state[d]);
    state[b] ^= state[c];
    state[b] = state[b].rotate_left(7);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ncbl_payload_has_v3_header_and_declared_length() {
        let payload = encrypt_ncbl("{}", "1\u{1}_pld\u{1}{}").unwrap();
        assert_eq!(&payload[..4], b"NCBL");
        assert_eq!(u32::from_le_bytes(payload[4..8].try_into().unwrap()), 3);
        let header_len = u16::from_le_bytes(payload[8..10].try_into().unwrap()) as usize;
        assert!(header_len >= 74 && header_len < payload.len());
    }

    #[test]
    fn chacha20_round_trip_restores_plaintext() {
        let key = [7_u8; 32];
        let nonce = [3_u8; 12];
        let encrypted = chacha20(&key, 42, &nonce, b"NeriPlayer scrobble");
        assert_eq!(
            chacha20(&key, 42, &nonce, &encrypted),
            b"NeriPlayer scrobble"
        );
    }
}
