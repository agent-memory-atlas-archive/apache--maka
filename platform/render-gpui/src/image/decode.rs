/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

//! Bounded static decoding, with original allocations retained by their permits.
use super::*;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use std::io::Cursor;

pub(super) const MAX_BYTES: usize = 5 * 1024 * 1024;
const MAX_EDGE: u32 = 8192;
const MAX_PIXELS: u64 = 4 * 1024 * 1024;
// Request the codec library's scratch limit per job. Enforcement is codec-dependent;
// this is not a process-wide allocator sandbox.
const MAX_WORK_BYTES: u64 = 64 * 1024 * 1024;
pub(super) struct Encoded {
    pub bytes: Vec<u8>,
    pub _permit: Permit,
}
pub(super) struct Decoded {
    pub image: Arc<RenderImage>,
    pub permit: Permit,
}
pub(super) fn decode(
    encoded: Encoded,
    source: ImageDescriptor,
    budget: NativeBudget,
    _job: Permit,
) -> Result<Decoded, ImageError> {
    if format!("sha256:{}", hex::encode(Sha256::digest(&encoded.bytes))) != source.digest {
        return Err(ImageError::DigestMismatch);
    }
    let format = match source.mime_type.as_str() {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::WebP,
        _ => return Err(ImageError::UnsupportedFormat),
    };
    if image::guess_format(&encoded.bytes).map_err(|_| ImageError::InvalidImage)? != format {
        return Err(ImageError::InvalidImage);
    }
    let mut reader = ImageReader::with_format(Cursor::new(&encoded.bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_EDGE);
    limits.max_image_height = Some(MAX_EDGE);
    limits.max_alloc = Some(MAX_WORK_BYTES);
    reader.limits(limits);
    let decoder = reader.into_decoder().map_err(classify)?;
    let (width, height) = decoder.dimensions();
    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .filter(|n| *n > 0 && *n <= MAX_PIXELS)
        .ok_or(ImageError::LimitExceeded)?;
    let permit = budget.reserve(
        Kind::Pixels,
        usize::try_from(pixels * 4).map_err(|_| ImageError::LimitExceeded)?,
    )?;
    // DynamicImage reads exactly one frame; no animation frame collection or URL loader.
    let mut rgba = DynamicImage::from_decoder(decoder)
        .map_err(classify)?
        .into_rgba8();
    if rgba.dimensions() != (width, height) || rgba.len() as u64 != pixels * 4 {
        return Err(ImageError::InvalidImage);
    }
    for pixel in rgba.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let image = Arc::new(RenderImage::new(vec![image::Frame::new(rgba)]));
    Ok(Decoded { image, permit })
}
fn classify(error: image::ImageError) -> ImageError {
    match error {
        image::ImageError::Limits(_) => ImageError::LimitExceeded,
        _ => ImageError::InvalidImage,
    }
}
