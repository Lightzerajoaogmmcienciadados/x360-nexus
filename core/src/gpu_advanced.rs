
use crate::gpu::Gpu;

#[derive(Clone, Copy)]
struct Vec3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vec3 {
    fn rotate(self, yaw: f32, pitch: f32) -> Self {
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();
        let x = self.x * cy + self.z * sy;
        let z = -self.x * sy + self.z * cy;
        Self {
            x,
            y: self.y * cp - z * sp,
            z: self.y * sp + z * cp,
        }
    }
}

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
    inv_z: f32,
    u: f32,
    v: f32,
}

pub struct AdvancedRenderer {
    width: usize,
    height: usize,
    depth: Vec<f32>,
}

impl AdvancedRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            depth: vec![f32::INFINITY; width * height],
        }
    }

    fn project(&self, v: Vec3) -> Option<Point> {
        if v.z <= 0.1 {
            return None;
        }
        let focal = self.height as f32 * 0.95;
        Some(Point {
            x: self.width as f32 * 0.5
                + v.x * focal / v.z,
            y: self.height as f32 * 0.5
                - v.y * focal / v.z,
            inv_z: 1.0 / v.z,
            u: 0.0,
            v: 0.0,
        })
    }

    fn edge(a: Point, b: Point, x: f32, y: f32) -> f32 {
        (x - a.x) * (b.y - a.y)
            - (y - a.y) * (b.x - a.x)
    }

    fn triangle(
        &mut self,
        gpu: &mut Gpu,
        a: Point,
        b: Point,
        c: Point,
        color: [u8; 3],
        light: f32,
    ) {
        let area = Self::edge(a, b, c.x, c.y);
        if area.abs() < 0.00001 {
            return;
        }

        let min_x = a.x.min(b.x).min(c.x)
            .floor().max(0.0) as usize;
        let min_y = a.y.min(b.y).min(c.y)
            .floor().max(0.0) as usize;
        let max_x = (a.x.max(b.x).max(c.x)
            .ceil().max(0.0) as usize)
            .min(self.width.saturating_sub(1));
        let max_y = (a.y.max(b.y).max(c.y)
            .ceil().max(0.0) as usize)
            .min(self.height.saturating_sub(1));

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = Self::edge(b, c, px, py) / area;
                let w1 = Self::edge(c, a, px, py) / area;
                let w2 = Self::edge(a, b, px, py) / area;

                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }

                let iz = w0 * a.inv_z
                    + w1 * b.inv_z
                    + w2 * c.inv_z;
                if iz <= 0.0 {
                    continue;
                }

                let z = 1.0 / iz;
                let index = y * self.width + x;
                if z >= self.depth[index] {
                    continue;
                }

                let u = (w0 * a.u * a.inv_z
                    + w1 * b.u * b.inv_z
                    + w2 * c.u * c.inv_z) / iz;
                let v = (w0 * a.v * a.inv_z
                    + w1 * b.v * b.inv_z
                    + w2 * c.v * c.inv_z) / iz;

                // Textura procedural xadrez.
                let checker =
                    ((u * 6.0).floor() as i32
                    + (v * 6.0).floor() as i32)
                    & 1;
                let texture = if checker == 0 {
                    1.0
                } else {
                    0.52
                };
                let intensity = (light * texture)
                    .clamp(0.0, 1.0);

                let rgba = [
                    (color[0] as f32 * intensity) as u8,
                    (color[1] as f32 * intensity) as u8,
                    (color[2] as f32 * intensity) as u8,
                    255,
                ];
                self.depth[index] = z;
                let _ = gpu.pixel(x, y, rgba);
            }
        }
    }

    fn cube(
        &mut self,
        gpu: &mut Gpu,
        center: Vec3,
        size: f32,
        yaw: f32,
        pitch: f32,
    ) {
        let vertices = [
            Vec3 { x: -1.0, y: -1.0, z: -1.0 },
            Vec3 { x:  1.0, y: -1.0, z: -1.0 },
            Vec3 { x:  1.0, y:  1.0, z: -1.0 },
            Vec3 { x: -1.0, y:  1.0, z: -1.0 },
            Vec3 { x: -1.0, y: -1.0, z:  1.0 },
            Vec3 { x:  1.0, y: -1.0, z:  1.0 },
            Vec3 { x:  1.0, y:  1.0, z:  1.0 },
            Vec3 { x: -1.0, y:  1.0, z:  1.0 },
        ];

        let transformed: Vec<Vec3> = vertices.iter()
            .map(|v| {
                let p = Vec3 {
                    x: v.x * size,
                    y: v.y * size,
                    z: v.z * size,
                }.rotate(yaw, pitch);
                Vec3 {
                    x: p.x + center.x,
                    y: p.y + center.y,
                    z: p.z + center.z,
                }
            })
            .collect();

        let faces: [([usize; 4], [u8; 3], Vec3); 6] = [
            ([0, 3, 2, 1], [0, 230, 95],
                Vec3 { x: 0.0, y: 0.0, z: -1.0 }),
            ([4, 5, 6, 7], [35, 120, 250],
                Vec3 { x: 0.0, y: 0.0, z: 1.0 }),
            ([0, 4, 7, 3], [255, 160, 40],
                Vec3 { x: -1.0, y: 0.0, z: 0.0 }),
            ([1, 2, 6, 5], [235, 55, 105],
                Vec3 { x: 1.0, y: 0.0, z: 0.0 }),
            ([3, 7, 6, 2], [190, 95, 255],
                Vec3 { x: 0.0, y: 1.0, z: 0.0 }),
            ([0, 1, 5, 4], [50, 220, 230],
                Vec3 { x: 0.0, y: -1.0, z: 0.0 }),
        ];

        for (ids, color, normal) in faces {
            let n = normal.rotate(yaw, pitch);
            let dot = (n.x * -0.4
                + n.y * 0.7
                + n.z * -0.6).max(0.0);
            let light = (0.25 + dot * 0.75).min(1.0);

            let mut projected = Vec::new();
            for id in ids {
                if let Some(p) = self.project(transformed[id]) {
                    projected.push(p);
                }
            }
            if projected.len() != 4 {
                continue;
            }

            let mut p = [
                projected[0], projected[1],
                projected[2], projected[3],
            ];
            p[0].u = 0.0; p[0].v = 0.0;
            p[1].u = 1.0; p[1].v = 0.0;
            p[2].u = 1.0; p[2].v = 1.0;
            p[3].u = 0.0; p[3].v = 1.0;

            self.triangle(gpu, p[0], p[1], p[2], color, light);
            self.triangle(gpu, p[0], p[2], p[3], color, light);
        }
    }

    pub fn render(
        &mut self,
        yaw: f32,
        pitch: f32,
    ) -> Result<Vec<u8>, &'static str> {
        let mut gpu = Gpu::new(self.width, self.height)?;
        gpu.clear([6, 13, 12, 255]);
        self.depth.fill(f32::INFINITY);

        self.cube(
            &mut gpu,
            Vec3 { x: -1.15, y: 0.0, z: 5.0 },
            0.85,
            yaw,
            pitch,
        );
        self.cube(
            &mut gpu,
            Vec3 { x: 1.15, y: 0.1, z: 6.0 },
            0.95,
            -yaw * 0.7,
            pitch * 0.8,
        );
        self.cube(
            &mut gpu,
            Vec3 { x: 0.0, y: -1.3, z: 8.0 },
            0.6,
            yaw * 1.3,
            pitch,
        );

        gpu.present();
        Ok(gpu.framebuffer().to_vec())
    }
}

pub fn render_advanced_frame(
    width: usize,
    height: usize,
    yaw: f32,
    pitch: f32,
) -> Result<Vec<u8>, &'static str> {
    if width == 0 || height == 0
        || !yaw.is_finite() || !pitch.is_finite() {
        return Err("invalid render parameters");
    }
    let mut renderer = AdvancedRenderer::new(width, height);
    renderer.render(yaw, pitch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advanced_frame_test() {
        let pixels = render_advanced_frame(
            640, 360, 0.4, 0.3
        ).unwrap();
        assert_eq!(pixels.len(), 640 * 360 * 4);
        assert!(pixels.chunks_exact(4).any(
            |p| p != [6, 13, 12, 255]
        ));
    }
}
