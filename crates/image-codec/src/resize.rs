use application::error::{ProcessingError as Error, Result};
use image::RgbaImage;
use std::sync::OnceLock;

#[derive(Clone, Copy)]
struct Span {
    start: u32,
    end: u32,
    first: f64,
    last: f64,
    extent: f64,
}

impl Span {
    fn new(source: u32, scale: f64, index: u32) -> Self {
        let left = f64::from(index) * scale;
        let right = f64::from(index + 1) * scale;
        let start = left.floor() as u32;
        let end = (right.ceil() as u32).min(source);
        Self {
            start,
            end,
            first: (right.min(f64::from(start + 1)) - left.max(f64::from(start))).max(0.),
            last: (right.min(f64::from(end)) - left.max(f64::from(end - 1))).max(0.),
            extent: right - left,
        }
    }

    fn weight(self, index: u32) -> f64 {
        // A one-pixel span uses its first weight once, even when both ends overlap it.
        if index == self.start {
            self.first
        } else if index + 1 == self.end {
            self.last
        } else {
            1.
        }
    }
}

struct Axis {
    source: u32,
    scale: f64,
    spans: Vec<Span>,
}

impl Axis {
    fn new(source: u32, output: u32) -> Self {
        Self {
            source,
            scale: f64::from(source) / f64::from(output),
            spans: Vec::new(),
        }
    }

    fn span(&self, index: u32) -> Span {
        self.spans
            .get(index as usize)
            .copied()
            .unwrap_or_else(|| Span::new(self.source, self.scale, index))
    }

    fn cache(&mut self, output: u32) -> Result<()> {
        self.spans
            .try_reserve_exact(output as usize)
            .map_err(|_| Error::ImageLimitsExceeded)?;
        for index in 0..output {
            self.spans.push(Span::new(self.source, self.scale, index));
        }
        Ok(())
    }
}

pub(crate) struct ResizePlan {
    source: (u32, u32),
    output: (u32, u32),
    x: Axis,
    y: Axis,
}

impl ResizePlan {
    pub(crate) fn new(source: (u32, u32), output: (u32, u32)) -> Self {
        Self {
            source,
            output,
            x: Axis::new(source.0, output.0),
            y: Axis::new(source.1, output.1),
        }
    }

    pub(crate) fn cache_bytes(output: (u32, u32)) -> Option<u64> {
        (u64::from(output.0) + u64::from(output.1))
            .checked_mul(std::mem::size_of::<Span>() as u64)?
            .checked_add(std::mem::size_of::<Self>() as u64)
    }

    pub(crate) fn cache(&mut self) -> Result<()> {
        self.x.cache(self.output.0)?;
        self.y.cache(self.output.1)
    }

    pub(crate) fn apply(&self, source: &RgbaImage) -> RgbaImage {
        assert_eq!(source.dimensions(), self.source);
        if self.source == self.output {
            return source.clone();
        }
        let tables = tables();
        let (width, height) = self.output;
        let stride = source.width() as usize * 4;
        let raw = source.as_raw();
        let mut output = RgbaImage::new(width, height);
        for (index, pixel) in output
            .as_mut()
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .enumerate()
        {
            let x = self.x.span((index % width as usize) as u32);
            let y = self.y.span((index / width as usize) as u32);
            let area = x.extent * y.extent;
            let mut sum = [0.; 4];
            // Keep the legacy source-Y/source-X order and (linear * alpha) * weight grouping.
            for sy in y.start..y.end {
                let wy = y.weight(sy);
                let row = &raw[sy as usize * stride..(sy as usize + 1) * stride];
                let pixels = &row[x.start as usize * 4..x.end as usize * 4];
                for (offset, source_pixel) in pixels.as_chunks::<4>().0.iter().enumerate() {
                    let weight = wy * x.weight(x.start + offset as u32);
                    let alpha = tables.alpha[usize::from(source_pixel[3])];
                    sum[0] += (tables.linear[usize::from(source_pixel[0])] * alpha) * weight;
                    sum[1] += (tables.linear[usize::from(source_pixel[1])] * alpha) * weight;
                    sum[2] += (tables.linear[usize::from(source_pixel[2])] * alpha) * weight;
                    sum[3] += alpha * weight;
                }
            }
            pixel[3] = (sum[3] / area * 255.).round().clamp(0., 255.) as u8;
            if sum[3] > 0. {
                pixel[0] = tables.encode(sum[0] / sum[3]);
                pixel[1] = tables.encode(sum[1] / sum[3]);
                pixel[2] = tables.encode(sum[2] / sum[3]);
            }
        }
        output
    }
}

struct TransferTables {
    linear: [f64; 256],
    alpha: [f64; 256],
    boundaries: [f64; 255],
    codes: [u8; 4096],
}

fn linear(srgb: f64) -> f64 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn legacy_encode(value: f64) -> u8 {
    let value = value.clamp(0., 1.);
    let srgb = if value <= 0.0031308 {
        value * 12.92
    } else {
        1.055 * value.powf(1. / 2.4) - 0.055
    };
    (srgb * 255.).round().clamp(0., 255.) as u8
}

impl TransferTables {
    fn encode(&self, value: f64) -> u8 {
        let value = value.clamp(0., 1.);
        let mut code = usize::from(self.codes[(value * 4095.) as usize]);
        while code > 0 && value < self.boundaries[code - 1] {
            code -= 1;
        }
        while code < 255 && value >= self.boundaries[code] {
            code += 1;
        }
        // Forward/inverse transfer rounding can disagree at a half-code boundary.
        if (code > 0 && (value - self.boundaries[code - 1]).abs() <= 1e-10)
            || (code < 255 && (value - self.boundaries[code]).abs() <= 1e-10)
        {
            legacy_encode(value)
        } else {
            code as u8
        }
    }
}

fn tables() -> &'static TransferTables {
    static TABLES: OnceLock<TransferTables> = OnceLock::new();
    TABLES.get_or_init(|| TransferTables {
        linear: std::array::from_fn(|index| linear(index as f64 / 255.)),
        alpha: std::array::from_fn(|index| index as f64 / 255.),
        boundaries: std::array::from_fn(|index| linear((index as f64 + 0.5) / 255.)),
        codes: std::array::from_fn(|index| legacy_encode(index as f64 / 4095.)),
    })
}

pub fn resize_inside(source: &RgbaImage, width: u32, height: u32) -> RgbaImage {
    if source.dimensions() == (width, height) {
        return source.clone();
    }
    ResizePlan::new(source.dimensions(), (width, height)).apply(source)
}

pub(crate) fn cached_plans(
    source: (u32, u32),
    outputs: &[(u32, u32)],
    source_reservation: u64,
    working_bytes: u64,
) -> Result<Option<Vec<ResizePlan>>> {
    if outputs.iter().all(|&output| output == source) {
        return Ok(None);
    }
    let Some(bytes) = outputs.iter().try_fold(0u64, |sum, &output| {
        sum.checked_add(ResizePlan::cache_bytes(output)?)
    }) else {
        return Ok(None);
    };
    if source_reservation
        .checked_add(bytes)
        .is_none_or(|total| total > working_bytes)
    {
        return Ok(None);
    }
    let mut plans = Vec::new();
    plans
        .try_reserve_exact(outputs.len())
        .map_err(|_| Error::ImageLimitsExceeded)?;
    for &output in outputs {
        let mut plan = ResizePlan::new(source, output);
        if output != source {
            plan.cache()?;
        }
        plans.push(plan);
    }
    Ok(Some(plans))
}

#[cfg(test)]
mod tests;
