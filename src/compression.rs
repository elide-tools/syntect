//! Compression backends for dump serialization/deserialization.
//!
//! This module provides pluggable compression backends selected via cargo features:
//! - `compression-flate2`: zlib/deflate compression (default, no dictionary)
//! - `compression-zstd`: zstd compression with dictionary support
//! - `compression-lz4`: lz4 compression with dictionary support
//!
//! Dictionary-based compression (zstd, lz4) provides better compression ratios
//! for the many small, structurally similar syntax definition blobs.
//!
//! If multiple compression features are enabled, priority is: zstd > lz4 > flate2

#[allow(unused_imports)]
use std::io::{Read, Write};

// Feature priority macros for cleaner cfg attributes
// Priority: zstd > lz4 > flate2

/// Embedded zstd dictionary for syntax compression.
#[cfg(feature = "compression-zstd")]
pub static ZSTD_DICTIONARY: &[u8] = include_bytes!("../assets/syntax_zstd.dict");

/// Embedded lz4 dictionary for syntax compression.
#[cfg(feature = "compression-lz4")]
pub static LZ4_DICTIONARY: &[u8] = include_bytes!("../assets/syntax_lz4.dict");

/// Compress data using the selected compression backend.
///
/// For dictionary-based backends (zstd, lz4), uses the embedded dictionary.
#[cfg(feature = "dump-create")]
pub fn compress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    compress_impl(data)
}

/// Decompress data using the selected compression backend.
///
/// For dictionary-based backends (zstd, lz4), uses the embedded dictionary.
#[cfg(feature = "dump-load")]
pub fn decompress(data: &[u8]) -> std::io::Result<Vec<u8>> {
    decompress_impl(data)
}

/// Compress data without using a dictionary.
///
/// Used for top-level structures where dictionary compression isn't beneficial.
#[cfg(feature = "dump-create")]
pub fn compress_no_dict(data: &[u8]) -> std::io::Result<Vec<u8>> {
    compress_no_dict_impl(data)
}

/// Decompress data that was compressed without a dictionary.
#[cfg(feature = "dump-load")]
pub fn decompress_no_dict(data: &[u8]) -> std::io::Result<Vec<u8>> {
    decompress_no_dict_impl(data)
}

// ============================================================================
// zstd backend (highest priority)
// ============================================================================

#[cfg(feature = "compression-zstd")]
use std::sync::LazyLock as Lazy;

#[cfg(all(feature = "compression-zstd", feature = "dump-create"))]
static ZSTD_ENCODER_DICT: Lazy<zstd::dict::EncoderDictionary<'static>> =
    Lazy::new(|| zstd::dict::EncoderDictionary::copy(ZSTD_DICTIONARY, 19));

#[cfg(all(feature = "compression-zstd", feature = "dump-load"))]
static ZSTD_DECODER_DICT: Lazy<zstd::dict::DecoderDictionary<'static>> =
    Lazy::new(|| zstd::dict::DecoderDictionary::copy(ZSTD_DICTIONARY));

#[cfg(all(feature = "compression-zstd", feature = "dump-create"))]
fn compress_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut encoder = zstd::Encoder::with_prepared_dictionary(&mut output, &*ZSTD_ENCODER_DICT)?;
    encoder.write_all(data)?;
    encoder.finish()?;
    Ok(output)
}

#[cfg(all(feature = "compression-zstd", feature = "dump-load"))]
fn decompress_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = zstd::Decoder::with_prepared_dictionary(data, &*ZSTD_DECODER_DICT)?;
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

#[cfg(all(feature = "compression-zstd", feature = "dump-create"))]
fn compress_no_dict_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    zstd::bulk::compress(data, 19)
}

#[cfg(all(feature = "compression-zstd", feature = "dump-load"))]
fn decompress_no_dict_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = zstd::Decoder::new(data)?;
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

// ============================================================================
// lz4 backend (second priority, only if zstd not enabled)
// ============================================================================

#[cfg(all(
    feature = "compression-lz4",
    not(feature = "compression-zstd"),
    feature = "dump-create"
))]
fn compress_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use lz4_flex::block::compress_prepend_size_with_dict;
    Ok(compress_prepend_size_with_dict(data, LZ4_DICTIONARY))
}

#[cfg(all(
    feature = "compression-lz4",
    not(feature = "compression-zstd"),
    feature = "dump-load"
))]
fn decompress_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use lz4_flex::block::decompress_size_prepended_with_dict;
    decompress_size_prepended_with_dict(data, LZ4_DICTIONARY)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[cfg(all(
    feature = "compression-lz4",
    not(feature = "compression-zstd"),
    feature = "dump-create"
))]
fn compress_no_dict_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use lz4_flex::block::compress_prepend_size;
    Ok(compress_prepend_size(data))
}

#[cfg(all(
    feature = "compression-lz4",
    not(feature = "compression-zstd"),
    feature = "dump-load"
))]
fn decompress_no_dict_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use lz4_flex::block::decompress_size_prepended;
    decompress_size_prepended(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

// ============================================================================
// flate2 backend (lowest priority, only if neither zstd nor lz4 enabled)
// ============================================================================

#[cfg(all(
    feature = "compression-flate2",
    not(feature = "compression-zstd"),
    not(feature = "compression-lz4"),
    feature = "dump-create"
))]
fn compress_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(data)?;
    encoder.finish()
}

#[cfg(all(
    feature = "compression-flate2",
    not(feature = "compression-zstd"),
    not(feature = "compression-lz4"),
    feature = "dump-load"
))]
fn decompress_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    use flate2::read::ZlibDecoder;

    let mut decoder = ZlibDecoder::new(data);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

#[cfg(all(
    feature = "compression-flate2",
    not(feature = "compression-zstd"),
    not(feature = "compression-lz4"),
    feature = "dump-create"
))]
fn compress_no_dict_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    compress_impl(data)
}

#[cfg(all(
    feature = "compression-flate2",
    not(feature = "compression-zstd"),
    not(feature = "compression-lz4"),
    feature = "dump-load"
))]
fn decompress_no_dict_impl(data: &[u8]) -> std::io::Result<Vec<u8>> {
    decompress_impl(data)
}

// ============================================================================
// Dictionary training (for gendata tool)
// ============================================================================

/// Train a zstd dictionary from sample data.
///
/// This is used during asset generation to create the embedded dictionary.
#[cfg(all(feature = "compression-zstd", feature = "dump-create"))]
pub fn train_zstd_dictionary(samples: &[Vec<u8>], dict_size: usize) -> std::io::Result<Vec<u8>> {
    let sample_refs: Vec<&[u8]> = samples.iter().map(|s| s.as_slice()).collect();
    zstd::dict::from_samples(&sample_refs, dict_size)
}

/// Train an lz4 dictionary from sample data.
///
/// LZ4 doesn't have built-in dictionary training, so we create a simple
/// dictionary by concatenating representative chunks from samples.
#[cfg(all(feature = "compression-lz4", feature = "dump-create"))]
pub fn train_lz4_dictionary(samples: &[Vec<u8>], dict_size: usize) -> Vec<u8> {
    // LZ4 dictionary is simpler - use representative data
    let mut dict = Vec::with_capacity(dict_size);
    let chunk_size = dict_size / samples.len().max(1);

    for sample in samples {
        let take = chunk_size.min(sample.len());
        dict.extend_from_slice(&sample[..take]);
        if dict.len() >= dict_size {
            break;
        }
    }

    dict.truncate(dict_size);
    dict
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #[cfg(all(
        feature = "compression-flate2",
        not(feature = "compression-zstd"),
        not(feature = "compression-lz4"),
        feature = "dump-create",
        feature = "dump-load"
    ))]
    #[test]
    fn flate2_roundtrip() {
        let data = b"Hello, world! This is test data for compression.".repeat(100);
        let compressed = super::compress_no_dict(&data).unwrap();
        let decompressed = super::decompress_no_dict(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[cfg(all(
        feature = "compression-zstd",
        feature = "dump-create",
        feature = "dump-load"
    ))]
    #[test]
    fn zstd_roundtrip_no_dict() {
        let data = b"Hello, world! This is test data for compression.".repeat(100);
        let compressed = super::compress_no_dict(&data).unwrap();
        let decompressed = super::decompress_no_dict(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[cfg(all(
        feature = "compression-lz4",
        not(feature = "compression-zstd"),
        feature = "dump-create",
        feature = "dump-load"
    ))]
    #[test]
    fn lz4_roundtrip_no_dict() {
        let data = b"Hello, world! This is test data for compression.".repeat(100);
        let compressed = super::compress_no_dict(&data).unwrap();
        let decompressed = super::decompress_no_dict(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }
}
