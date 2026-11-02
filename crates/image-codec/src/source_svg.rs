use crate::source::{Description, Emit, static_facts};
use application::{
    error::{ProcessingError as Error, Result},
    images::{Limits, SourceFormat},
};
use image::{DynamicImage, RgbaImage, metadata::Orientation};
use resvg::{tiny_skia, usvg};
use std::{
    io::Cursor,
    sync::{Arc, Mutex},
};

#[derive(Default)]
struct EmbeddedState {
    bytes: u64,
    error: Option<Error>,
}

pub(crate) fn decode(bytes: &[u8], limits: &Limits, emit: &mut Emit<'_>) -> Result<Description> {
    let text = std::str::from_utf8(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes))
        .map_err(|_| Error::InvalidImage)?;
    let xml = roxmltree::Document::parse_with_options(
        text,
        roxmltree::ParsingOptions {
            allow_dtd: false,
            nodes_limit: (limits.working_bytes / 64).min(u64::from(u32::MAX)) as u32,
            ..Default::default()
        },
    )
    .map_err(|_| Error::InvalidImage)?;
    if xml.root_element().tag_name().name() != "svg" {
        return Err(Error::UnsupportedFormat);
    }
    if xml
        .descendants()
        .any(|n| n.pi().is_some_and(|pi| pi.target == "xml-stylesheet"))
    {
        return Err(Error::UnsupportedFormat);
    }
    for node in xml.descendants().filter(|n| n.is_element()) {
        match node.tag_name().name() {
            "animate" | "animateMotion" | "animateTransform" | "set" | "discard" | "script" => {
                return Err(Error::UnrepresentableAnimation);
            }
            "foreignObject" => return Err(Error::UnsupportedFormat),
            _ => {}
        }
        for attr in node.attributes() {
            if attr.name().starts_with("on") {
                return Err(Error::UnrepresentableAnimation);
            }
            if attr.name() == "href" {
                match node.tag_name().name() {
                    "a" => {}
                    "image" if attr.value().starts_with("data:") => {}
                    "image" => return Err(Error::UnsupportedFormat),
                    "feImage"
                        if attr.value().starts_with("data:") || attr.value().starts_with('#') => {}
                    _ if attr.value().starts_with('#') => {}
                    _ => return Err(Error::UnsupportedFormat),
                }
            }
            if attr.name() == "style"
                || attr.value().to_ascii_lowercase().contains("url(")
                || attr.value().contains('\\')
            {
                validate_css(attr.value())?;
            }
        }
        if node.tag_name().name() == "style" {
            let content: String = node
                .children()
                .filter(|n| n.is_text())
                .filter_map(|n| n.text())
                .collect();
            validate_css(&content)?;
        }
    }
    let state = Arc::new(Mutex::new(EmbeddedState::default()));
    let captured = state.clone();
    let embedded_limits = limits.clone();
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    // Resolve embedded rasters through the same orientation and color path as top-level images.
    options.image_href_resolver.resolve_data = Box::new(move |_, data, _| {
        let result = embedded_png(&data, &embedded_limits, &captured);
        match result {
            Ok(png) => Some(usvg::ImageKind::PNG(Arc::new(png))),
            Err(error) => {
                captured
                    .lock()
                    .expect("embedded state")
                    .error
                    .get_or_insert(error);
                None
            }
        }
    });
    options.image_href_resolver.resolve_string = Box::new(|_, _| None);
    let tree = usvg::Tree::from_str(text, &options).map_err(|_| Error::InvalidImage)?;
    if let Some(error) = state.lock().map_err(|_| Error::InternalError)?.error {
        return Err(error);
    }
    let width = tree.size().width().ceil() as u32;
    let height = tree.size().height().ceil() as u32;
    let facts = static_facts(width, height);
    limits.preflight(&facts)?;
    let embedded_bytes = state.lock().map_err(|_| Error::InternalError)?.bytes;
    if u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|n| n.checked_mul(20))
        .and_then(|n| n.checked_add(embedded_bytes))
        .is_none_or(|n| n > limits.working_bytes)
    {
        return Err(Error::ImageLimitsExceeded);
    }
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or(Error::ImageLimitsExceeded)?;
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / tree.size().width(),
        height as f32 / tree.size().height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    for pixel in pixmap.data_mut().as_chunks_mut::<4>().0.iter_mut() {
        let alpha = u32::from(pixel[3]);
        if let Some(alpha) = std::num::NonZeroU32::new(alpha) {
            let alpha = alpha.get();
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
    let pixels = RgbaImage::from_raw(width, height, pixmap.take()).ok_or(Error::InternalError)?;
    emit(pixels, 0, Orientation::NoTransforms, None)?;
    Ok((SourceFormat::Svg, facts, 1))
}

fn validate_css(css: &str) -> Result<()> {
    if css.contains('\\') {
        return Err(Error::UnsupportedFormat);
    }
    let css = css.to_ascii_lowercase();
    if [
        "@keyframes",
        "@-webkit-keyframes",
        "@-moz-keyframes",
        "@-o-keyframes",
        "@-ms-keyframes",
    ]
    .iter()
    .any(|keyword| css.contains(keyword))
    {
        return Err(Error::UnrepresentableAnimation);
    }
    for declaration in css.split(['{', '}', ';']) {
        if let Some((name, _)) = declaration.split_once(':') {
            let mut name = name.trim();
            for prefix in ["-webkit-", "-moz-", "-o-", "-ms-"] {
                if let Some(property) = name.strip_prefix(prefix) {
                    name = property;
                    break;
                }
            }
            if name.starts_with("animation")
                || name.starts_with("transition")
                || name.starts_with("-webkit-animation")
                || name.starts_with("-webkit-transition")
            {
                return Err(Error::UnrepresentableAnimation);
            }
        }
    }
    if css.contains("@import") {
        return Err(Error::UnsupportedFormat);
    }
    for url in css.split("url(").skip(1) {
        let value = url
            .split(')')
            .next()
            .ok_or(Error::InvalidImage)?
            .trim()
            .trim_matches(['\'', '"']);
        if !value.starts_with('#') {
            return Err(Error::UnsupportedFormat);
        }
    }
    Ok(())
}

fn embedded_png(data: &[u8], limits: &Limits, state: &Mutex<EmbeddedState>) -> Result<Vec<u8>> {
    if data.len() as u64 > limits.source_bytes {
        return Err(Error::ImageLimitsExceeded);
    }
    if !(data.starts_with(b"\x89PNG")
        || data.starts_with(b"GIF8")
        || data.starts_with(b"RIFF")
        || data.starts_with(&[255, 216]))
    {
        return Err(Error::UnsupportedFormat);
    }
    let mut png = None;
    let decoded = crate::source::decode_source(
        data,
        limits,
        None,
        || {},
        |pixels, delay, _| {
            if delay > 0 {
                return Err(Error::UnrepresentableAnimation);
            }
            let mut guard = state.lock().map_err(|_| Error::InternalError)?;
            guard.bytes = guard
                .bytes
                .checked_add(pixels.as_raw().len() as u64)
                .ok_or(Error::ImageLimitsExceeded)?;
            if guard.bytes > limits.working_bytes || guard.bytes > limits.decoded_bytes {
                return Err(Error::ImageLimitsExceeded);
            }
            drop(guard);
            let mut output = Cursor::new(Vec::new());
            DynamicImage::ImageRgba8(pixels)
                .write_to(&mut output, image::ImageFormat::Png)
                .map_err(|_| Error::InternalError)?;
            png = Some(output.into_inner());
            Ok(())
        },
    )?;
    if decoded.facts.image.animated {
        return Err(Error::UnrepresentableAnimation);
    }
    png.ok_or(Error::InvalidImage)
}
