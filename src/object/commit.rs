use indexmap::IndexMap;
use std::io;
pub struct CommitObject {
    pub kvlm: IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>,
}

impl CommitObject {
    pub fn new(kvlm: IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>) -> Self {
        Self { kvlm }
    }

    pub fn get_kvlm(&self) -> &IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>> {
        &self.kvlm
    }
}

pub fn kvlm_parse(
    raw: &[u8],
    start: Option<usize>,
    dct: Option<IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>>,
) -> io::Result<IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>> {
    let start = start.unwrap_or(0);
    let mut dct = dct.unwrap_or_default();

    let spc: Option<usize> = raw[start..]
        .iter()
        .position(|&b| b == b' ')
        .map(|p| p + start);

    let nl = raw[start..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|p| p + start);

    let is_message_body = match (spc, nl) {
        (None, _) => true,
        (Some(s), Some(n)) if n < s => true,
        _ => false,
    };

    if is_message_body {
        let nl_pos =
            nl.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing newline"))?;

        if nl_pos != start {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Malformed header: newline not at start",
            ));
        }

        let mut msg: Vec<Vec<u8>> = Vec::new();

        msg.push(raw[start + 1..].to_vec());

        dct.insert(None, msg);
        return Ok(dct);
    }

    let spc = spc.unwrap();

    let key = raw[start..spc].to_vec();

    let end = raw[start..]
        .windows(2)
        .position(|w| w[0] == b'\n' && w[1] != b' ')
        .map(|p| p + start)
        .unwrap();

    let value = raw[spc + 1..end].to_vec();

    dct.entry(Some(key)).or_insert_with(Vec::new).push(value);

    kvlm_parse(raw, Some(end + 1), Some(dct))
}

pub fn kvlm_serialize(kvlm: &IndexMap<Option<Vec<u8>>, Vec<Vec<u8>>>) -> Vec<u8> {
    let mut ret = Vec::new();

    for (key, value) in kvlm {
        let k = match key {
            Some(k) => k,
            None => continue,
        };

        for v in value {
            let v_processed: Vec<u8> = v
                .iter()
                .flat_map(|&b| {
                    if b == b'\n' {
                        vec![b'\n', b' ']
                    } else {
                        vec![b]
                    }
                })
                .collect();
            ret.extend_from_slice(k);
            ret.push(b' ');
            ret.extend_from_slice(&v_processed);
            ret.push(b'\n');
        }
    }
    ret.push(b'\n');
    if let Some(msg) = kvlm.get(&None) {
        ret.extend_from_slice(&msg[0]);
    }
    ret
}
