
#[derive(Debug)]
pub struct Gpu {
    pub width: usize,
    pub height: usize,
    rgba: Vec<u8>,
    frames: u64,
}

impl Gpu {
    pub fn new(
        width: usize,
        height: usize,
    ) -> Result<Self, &'static str> {
        if width == 0 || height == 0 {
            return Err("invalid framebuffer size");
        }

        let len = width
            .checked_mul(height)
            .and_then(|v| v.checked_mul(4))
            .ok_or("framebuffer overflow")?;

        if len > 64 * 1024 * 1024 {
            return Err("framebuffer too large");
        }

        Ok(Self {
            width,
            height,
            rgba: vec![0; len],
            frames: 0,
        })
    }

    pub fn clear(&mut self, color: [u8; 4]) {
        for pixel in self.rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&color);
        }
    }

    pub fn pixel(
        &mut self,
        x: usize,
        y: usize,
        color: [u8; 4],
    ) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            return Err("pixel out of bounds");
        }

        let index = (y * self.width + x) * 4;
        self.rgba[index..index + 4]
            .copy_from_slice(&color);

        Ok(())
    }

    pub fn get_pixel(
        &self,
        x: usize,
        y: usize,
    ) -> Result<[u8; 4], &'static str> {
        if x >= self.width || y >= self.height {
            return Err("pixel out of bounds");
        }

        let index = (y * self.width + x) * 4;

        Ok([
            self.rgba[index],
            self.rgba[index + 1],
            self.rgba[index + 2],
            self.rgba[index + 3],
        ])
    }

    pub fn rectangle(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: [u8; 4],
    ) -> Result<(), &'static str> {
        let right = x.checked_add(width)
            .ok_or("rectangle overflow")?;

        let bottom = y.checked_add(height)
            .ok_or("rectangle overflow")?;

        if right > self.width || bottom > self.height {
            return Err("rectangle out of bounds");
        }

        for py in y..bottom {
            for px in x..right {
                self.pixel(px, py, color)?;
            }
        }

        Ok(())
    }

    pub fn gradient(&mut self) {
        let width = self.width.max(1);
        let height = self.height.max(1);

        for y in 0..self.height {
            for x in 0..self.width {
                let r = ((x * 255) / width) as u8;
                let g = ((y * 255) / height) as u8;

                let _ = self.pixel(
                    x,
                    y,
                    [r, g, 80, 255],
                );
            }
        }
    }

    pub fn render_test(&mut self) {
        self.clear([8, 14, 12, 255]);
        self.gradient();

        let w = self.width / 3;
        let h = self.height / 3;

        let _ = self.rectangle(
            w,
            h,
            w,
            h,
            [0, 220, 80, 255],
        );

        self.present();
    }

    pub fn present(&mut self) {
        self.frames = self.frames.wrapping_add(1);
    }

    pub fn frame_count(&self) -> u64 {
        self.frames
    }

    pub fn framebuffer(&self) -> &[u8] {
        &self.rgba
    }

    pub fn framebuffer_size(&self) -> usize {
        self.rgba.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_test() {
        let mut gpu = Gpu::new(2, 1).unwrap();
        gpu.clear([0, 255, 0, 255]);

        assert_eq!(
            gpu.framebuffer(),
            &[0, 255, 0, 255, 0, 255, 0, 255]
        );
    }

    #[test]
    fn pixel_test() {
        let mut gpu = Gpu::new(4, 4).unwrap();

        gpu.pixel(2, 1, [255, 0, 0, 255])
            .unwrap();

        assert_eq!(
            gpu.get_pixel(2, 1).unwrap(),
            [255, 0, 0, 255]
        );
    }

    #[test]
    fn rectangle_test() {
        let mut gpu = Gpu::new(8, 8).unwrap();

        gpu.rectangle(
            2, 2, 3, 3,
            [0, 255, 0, 255],
        ).unwrap();

        assert_eq!(
            gpu.get_pixel(3, 3).unwrap(),
            [0, 255, 0, 255]
        );

        assert!(gpu.pixel(10, 10, [0; 4]).is_err());
    }

    #[test]
    fn render_test_frame() {
        let mut gpu = Gpu::new(640, 360).unwrap();

        gpu.render_test();

        assert_eq!(gpu.frame_count(), 1);
        assert_eq!(
            gpu.framebuffer_size(),
            640 * 360 * 4
        );
    }
}
