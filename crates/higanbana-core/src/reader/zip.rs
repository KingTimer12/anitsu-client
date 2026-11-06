//! Leitura de `.cbz`/`.zip` por pedaços, sem baixar o arquivo inteiro.
//!
//! O índice de um zip (diretório central) fica no fim do arquivo. Lendo só a
//! cauda achamos onde ele está; com ele, cada imagem é um intervalo de bytes
//! que dá para pedir com `Range` e descompactar sozinho.

use std::io::Read;

use thiserror::Error;

const EOCD_SIG: u32 = 0x0605_4b50;
const ZIP64_LOCATOR_SIG: u32 = 0x0706_4b50;
const ZIP64_EOCD_SIG: u32 = 0x0606_4b50;
const CD_SIG: u32 = 0x0201_4b50;
const LOCAL_SIG: u32 = 0x0403_4b50;
const EOCD_LEN: usize = 22;
const ZIP64_LOCATOR_LEN: usize = 20;
const LOCAL_HEADER_LEN: u64 = 30;
/// Folga para o "extra" do cabeçalho local, que pode diferir do diretório central.
pub const LOCAL_EXTRA_GUESS: u64 = 256;

#[derive(Debug, Error, PartialEq)]
pub enum ZipError {
    #[error("o arquivo não é um zip/cbz válido")]
    NotZip,
    #[error("o arquivo está corrompido ou incompleto")]
    Corrupt,
    #[error("compressão {0} não suportada")]
    UnsupportedMethod(u16),
}

/// Quantos bytes do fim buscar para achar o fim do diretório central
/// (registro de 22 bytes + comentário de até 64 KiB + localizador zip64).
pub fn tail_len(file_size: u64) -> u64 {
    file_size.min((EOCD_LEN + u16::MAX as usize + ZIP64_LOCATOR_LEN) as u64)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CdLocation {
    pub offset: u64,
    pub size: u64,
}

#[derive(Debug, PartialEq)]
pub enum Located {
    Found(CdLocation),
    /// Zip64 cujo registro final não veio na cauda: buscar 56 bytes neste offset.
    Zip64At(u64),
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?))
}
fn u64_at(b: &[u8], i: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(i..i + 8)?.try_into().ok()?))
}

/// Acha o diretório central na cauda do arquivo. `tail_start` é o offset do
/// primeiro byte de `tail` no arquivo.
pub fn locate_cd(tail: &[u8], tail_start: u64) -> Result<Located, ZipError> {
    if tail.len() < EOCD_LEN {
        return Err(ZipError::NotZip);
    }
    let eocd = (0..=tail.len() - EOCD_LEN)
        .rev()
        .find(|&i| u32_at(tail, i) == Some(EOCD_SIG))
        .ok_or(ZipError::NotZip)?;
    let size = u32_at(tail, eocd + 12).ok_or(ZipError::Corrupt)?;
    let offset = u32_at(tail, eocd + 16).ok_or(ZipError::Corrupt)?;
    if size != u32::MAX && offset != u32::MAX {
        return Ok(Located::Found(CdLocation { offset: offset as u64, size: size as u64 }));
    }
    // Zip64: o localizador fica logo antes do registro final.
    let loc = eocd.checked_sub(ZIP64_LOCATOR_LEN).ok_or(ZipError::Corrupt)?;
    if u32_at(tail, loc) != Some(ZIP64_LOCATOR_SIG) {
        return Err(ZipError::Corrupt);
    }
    let record = u64_at(tail, loc + 8).ok_or(ZipError::Corrupt)?;
    match record.checked_sub(tail_start) {
        Some(rel) if (rel as usize) + 56 <= tail.len() => parse_zip64_eocd(&tail[rel as usize..]).map(Located::Found),
        _ => Ok(Located::Zip64At(record)),
    }
}

/// Registro final zip64 (56 bytes).
pub fn parse_zip64_eocd(b: &[u8]) -> Result<CdLocation, ZipError> {
    if u32_at(b, 0) != Some(ZIP64_EOCD_SIG) {
        return Err(ZipError::Corrupt);
    }
    Ok(CdLocation {
        size: u64_at(b, 40).ok_or(ZipError::Corrupt)?,
        offset: u64_at(b, 48).ok_or(ZipError::Corrupt)?,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub method: u16,
    pub compressed_size: u64,
    pub size: u64,
    pub local_offset: u64,
    /// Onde os dados começam, quando o formato já informa (RAR). No zip, `None`:
    /// sai do cabeçalho local em [`extract`].
    pub data_offset: Option<u64>,
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        self.name.ends_with('/')
    }
    /// Intervalo (inclusivo) que cobre cabeçalho local + dados, com folga para o "extra".
    pub fn fetch_range(&self) -> (u64, u64) {
        let len = LOCAL_HEADER_LEN + self.name.len() as u64 + LOCAL_EXTRA_GUESS + self.compressed_size;
        (self.local_offset, self.local_offset + len - 1)
    }
}

/// Entradas do diretório central (com tamanhos e offsets zip64 quando houver).
pub fn parse_cd(b: &[u8]) -> Result<Vec<Entry>, ZipError> {
    let mut out = Vec::new();
    let mut i = 0;
    while i + 46 <= b.len() {
        if u32_at(b, i) != Some(CD_SIG) {
            return Err(ZipError::Corrupt);
        }
        let c = || ZipError::Corrupt;
        let method = u16_at(b, i + 10).ok_or_else(c)?;
        let mut compressed_size = u32_at(b, i + 20).ok_or_else(c)? as u64;
        let mut size = u32_at(b, i + 24).ok_or_else(c)? as u64;
        let name_len = u16_at(b, i + 28).ok_or_else(c)? as usize;
        let extra_len = u16_at(b, i + 30).ok_or_else(c)? as usize;
        let comment_len = u16_at(b, i + 32).ok_or_else(c)? as usize;
        let mut local_offset = u32_at(b, i + 42).ok_or_else(c)? as u64;
        let name_start = i + 46;
        let name = b.get(name_start..name_start + name_len).ok_or_else(c)?;
        let extra = b.get(name_start + name_len..name_start + name_len + extra_len).ok_or_else(c)?;

        // Campo extra zip64 (0x0001): só traz os valores que estouraram, nesta ordem.
        let mut j = 0;
        while j + 4 <= extra.len() {
            let id = u16_at(extra, j).ok_or_else(c)?;
            let len = u16_at(extra, j + 2).ok_or_else(c)? as usize;
            if id == 1 {
                let mut k = j + 4;
                for field in [&mut size, &mut compressed_size, &mut local_offset] {
                    if *field == u32::MAX as u64 {
                        *field = u64_at(extra, k).ok_or_else(c)?;
                        k += 8;
                    }
                }
            }
            j += 4 + len;
        }

        out.push(Entry {
            name: String::from_utf8_lossy(name).into_owned(),
            method,
            compressed_size,
            size,
            local_offset,
            data_offset: None,
        });
        i = name_start + name_len + extra_len + comment_len;
    }
    Ok(out)
}

/// Extrai os dados de uma entrada a partir dos bytes que começam no cabeçalho local.
/// `Ok(Err(fim))` = faltaram bytes (o "extra" local era maior que a folga): buscar até `fim` (exclusivo).
pub fn extract(entry: &Entry, local: &[u8]) -> Result<Result<Vec<u8>, u64>, ZipError> {
    if u32_at(local, 0) != Some(LOCAL_SIG) {
        return Err(ZipError::Corrupt);
    }
    let name_len = u16_at(local, 26).ok_or(ZipError::Corrupt)? as u64;
    let extra_len = u16_at(local, 28).ok_or(ZipError::Corrupt)? as u64;
    let start = LOCAL_HEADER_LEN + name_len + extra_len;
    let end = start + entry.compressed_size;
    if (local.len() as u64) < end {
        return Ok(Err(end));
    }
    let data = &local[start as usize..end as usize];
    match entry.method {
        0 => Ok(Ok(data.to_vec())),
        8 => {
            let mut out = Vec::with_capacity(entry.size as usize);
            flate2::read::DeflateDecoder::new(data)
                .read_to_end(&mut out)
                .map_err(|_| ZipError::Corrupt)?;
            Ok(Ok(out))
        }
        m => Err(ZipError::UnsupportedMethod(m)),
    }
}

/// Onde começam os dados de uma entrada, a partir dos 30 bytes do cabeçalho local.
pub fn local_data_offset(header: &[u8]) -> Result<u64, ZipError> {
    if u32_at(header, 0) != Some(LOCAL_SIG) {
        return Err(ZipError::Corrupt);
    }
    let name_len = u16_at(header, 26).ok_or(ZipError::Corrupt)? as u64;
    let extra_len = u16_at(header, 28).ok_or(ZipError::Corrupt)? as u64;
    Ok(LOCAL_HEADER_LEN + name_len + extra_len)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// Monta um zip simples em memória: (nome, conteúdo, deflate?).
    pub fn build_zip(files: &[(&str, &[u8], bool)], comment: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut cd = Vec::new();
        for (name, data, deflate) in files {
            let offset = out.len() as u32;
            let (method, body) = if *deflate {
                let mut e = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
                e.write_all(data).unwrap();
                (8u16, e.finish().unwrap())
            } else {
                (0u16, data.to_vec())
            };
            let local_extra = [0xAAu8; 9]; // "extra" local diferente do central
            out.extend(LOCAL_SIG.to_le_bytes());
            out.extend([20, 0, 0, 0]);
            out.extend(method.to_le_bytes());
            out.extend([0u8; 8]); // hora, data, crc
            out.extend((body.len() as u32).to_le_bytes());
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend((local_extra.len() as u16).to_le_bytes());
            out.extend(name.as_bytes());
            out.extend(local_extra);
            out.extend(&body);

            cd.extend(CD_SIG.to_le_bytes());
            cd.extend([20, 0, 20, 0, 0, 0]);
            cd.extend(method.to_le_bytes());
            cd.extend([0u8; 8]);
            cd.extend((body.len() as u32).to_le_bytes());
            cd.extend((data.len() as u32).to_le_bytes());
            cd.extend((name.len() as u16).to_le_bytes());
            cd.extend([0u8; 12]); // extra, comentário, disco, atributos
            cd.extend(offset.to_le_bytes());
            cd.extend(name.as_bytes());
        }
        let cd_offset = out.len() as u32;
        out.extend(&cd);
        out.extend(EOCD_SIG.to_le_bytes());
        out.extend([0u8; 4]);
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((files.len() as u16).to_le_bytes());
        out.extend((cd.len() as u32).to_le_bytes());
        out.extend(cd_offset.to_le_bytes());
        out.extend((comment.len() as u16).to_le_bytes());
        out.extend(comment);
        out
    }

    fn read_all(zip: &[u8]) -> Vec<(String, Vec<u8>)> {
        let tail_start = zip.len() as u64 - tail_len(zip.len() as u64);
        let Located::Found(loc) = locate_cd(&zip[tail_start as usize..], tail_start).unwrap() else {
            panic!("zip64 inesperado")
        };
        let cd = &zip[loc.offset as usize..(loc.offset + loc.size) as usize];
        parse_cd(cd)
            .unwrap()
            .into_iter()
            .map(|e| {
                let (a, b) = e.fetch_range();
                let slice = &zip[a as usize..=(b as usize).min(zip.len() - 1)];
                (e.name.clone(), extract(&e, slice).unwrap().unwrap())
            })
            .collect()
    }

    #[test]
    fn reads_stored_and_deflated_entries() {
        let page = vec![7u8; 5000];
        let zip = build_zip(&[("001.jpg", b"jpeg-bytes", false), ("002.png", &page, true)], b"");
        let all = read_all(&zip);
        assert_eq!(all[0], ("001.jpg".into(), b"jpeg-bytes".to_vec()));
        assert_eq!(all[1], ("002.png".into(), page));
    }

    #[test]
    fn finds_eocd_behind_comment() {
        let zip = build_zip(&[("a.jpg", b"x", false)], &[b'c'; 3000]);
        assert_eq!(read_all(&zip)[0].1, b"x");
    }

    #[test]
    fn local_data_offset_skips_name_and_extra() {
        let zip = build_zip(&[("Cap 1/001.jpg", b"x", false)], b"");
        // build_zip grava 9 bytes de "extra" local.
        assert_eq!(local_data_offset(&zip[..30]), Ok(30 + 13 + 9));
        assert_eq!(local_data_offset(&[0u8; 30]), Err(ZipError::Corrupt));
    }

    #[test]
    fn reads_zip_stored_inside_zip() {
        let inner = build_zip(&[("001.jpg", b"pagina-1", false), ("002.jpg", b"pagina-2", true)], b"");
        let outer = build_zip(&[("Capitulo 1.cbz", &inner, false)], b"");
        let tail_start = outer.len() as u64 - tail_len(outer.len() as u64);
        let Located::Found(loc) = locate_cd(&outer[tail_start as usize..], tail_start).unwrap() else { panic!() };
        let entry = parse_cd(&outer[loc.offset as usize..(loc.offset + loc.size) as usize]).unwrap().remove(0);
        let start = entry.local_offset as usize;
        let base = start + local_data_offset(&outer[start..start + 30]).unwrap() as usize;
        let window = &outer[base..base + entry.compressed_size as usize];
        let pages = read_all(window);
        assert_eq!(pages[1], ("002.jpg".into(), b"pagina-2".to_vec()));
    }

    #[test]
    fn rejects_non_zip() {
        assert_eq!(locate_cd(&[1u8; 100], 0), Err(ZipError::NotZip));
        assert_eq!(locate_cd(b"Rar!", 0), Err(ZipError::NotZip));
    }

    #[test]
    fn asks_for_more_when_local_extra_is_large() {
        let e = Entry { name: "a.jpg".into(), method: 0, compressed_size: 4, size: 4, local_offset: 0, data_offset: None };
        let mut local = Vec::new();
        local.extend(LOCAL_SIG.to_le_bytes());
        local.extend([0u8; 22]);
        local.extend(5u16.to_le_bytes());
        local.extend(1000u16.to_le_bytes());
        local.extend(b"a.jpg");
        assert_eq!(extract(&e, &local).unwrap(), Err(30 + 5 + 1000 + 4));
    }

    #[test]
    fn unsupported_method() {
        let zip = build_zip(&[("a.jpg", b"x", false)], b"");
        let mut e = parse_cd(&zip[zip.len() - 22 - 46 - 5..zip.len() - 22]).unwrap().remove(0);
        e.method = 12; // bzip2
        let (a, b) = e.fetch_range();
        let slice = &zip[a as usize..=(b as usize).min(zip.len() - 1)];
        assert_eq!(extract(&e, slice), Err(ZipError::UnsupportedMethod(12)));
    }

    #[test]
    fn zip64_extra_field() {
        let mut cd = Vec::new();
        cd.extend(CD_SIG.to_le_bytes());
        cd.extend([0u8; 6]);
        cd.extend(8u16.to_le_bytes());
        cd.extend([0u8; 8]);
        cd.extend(u32::MAX.to_le_bytes()); // compressed
        cd.extend(u32::MAX.to_le_bytes()); // size
        cd.extend(1u16.to_le_bytes()); // nome
        cd.extend(28u16.to_le_bytes()); // extra
        cd.extend([0u8; 10]);
        cd.extend(u32::MAX.to_le_bytes()); // offset
        cd.extend(b"p");
        cd.extend(1u16.to_le_bytes());
        cd.extend(24u16.to_le_bytes());
        cd.extend(6_000_000_000u64.to_le_bytes());
        cd.extend(5_000_000_000u64.to_le_bytes());
        cd.extend(4_500_000_000u64.to_le_bytes());
        let e = parse_cd(&cd).unwrap().remove(0);
        assert_eq!((e.size, e.compressed_size, e.local_offset), (6_000_000_000, 5_000_000_000, 4_500_000_000));
    }

    #[test]
    fn zip64_record_outside_tail() {
        let mut tail = Vec::new();
        tail.extend(ZIP64_LOCATOR_SIG.to_le_bytes());
        tail.extend([0u8; 4]);
        tail.extend(9_000u64.to_le_bytes());
        tail.extend([0u8; 4]);
        tail.extend(EOCD_SIG.to_le_bytes());
        tail.extend([0u8; 8]);
        tail.extend(u32::MAX.to_le_bytes());
        tail.extend(u32::MAX.to_le_bytes());
        tail.extend([0u8; 2]);
        assert_eq!(locate_cd(&tail, 100_000), Ok(Located::Zip64At(9_000)));
    }
}
