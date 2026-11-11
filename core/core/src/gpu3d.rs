use crate::gpu::Gpu;

#[derive(Clone, Copy)]
struct Vertex {
    x: f32,
    y: f32,
    z: f32,
}

pub struct Renderer3D {
    depth: Vec<f32>,
    width: usize,
    height: usize,
}

impl Renderer3D {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            depth: vec![f32::INFINITY; width * height],
            width,
            height,
        }
    }

    fn rotate(v: Vertex, yaw: f32, pitch: f32) -> Vertex {
        let (sy, cy) = yaw.sin_cos();
        let (sp, cp) = pitch.sin_cos();

        let x = v.x * cy + v.z * sy;
        let z = -v.x * sy + v.z * cy;

        Vertex {
            x,
            y: v.y * cp - z * sp,
            z: v.y * sp + z * cp,
        }
    }

    fn project(&self, v: Vertex) -> Vertex {
        let z = v.z + 4.0;
        let scale = self.height as f32 * 0.85;

        Vertex {
            x: self.width as f32 * 0.5 + v.x * scale / z,
            y: self.height as f32 * 0.5 - v.y * scale / z,
            z,
        }
    }

    fn edge(a: Vertex, b: Vertex, p: Vertex) -> f32 {
        (p.x - a.x) * (b.y - a.y)
            - (p.y - a.y) * (b.x - a.x)
    }

    fn triangle(
        &mut self,
        gpu: &mut Gpu,
        a: Vertex,
        b: Vertex,
        c: Vertex,
        color: [u8; 4],
    ) {
        let area = Self::edge(a, b, c);

        if area.abs() < 0.00001 {
            return;
        }

        let min_x = a.x.min(b.x).min(c.x)
            .floor().max(0.0) as usize;
        let min_y = a.y.min(b.y).min(c.y)
            .floor().max(0.0) as usize;

        let max_x = (a.x.max(b.x).max(c.x)
            .ceil() as usize)
            .min(self.width.saturating_sub(1));
        let max_y = (a.y.max(b.y).max(c.y)
            .ceil() as usize)
            .min(self.height.saturating_sub(1));

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = Vertex {
                    x: x as f32 + 0.5,
                    y: y as f32 + 0.5,
                    z: 0.0,
                };

                let w0 = Self::edge(b, c, p) / area;
                let w1 = Self::edge(c, a, p) / area;
                let w2 = Self::edge(a, b, p) / area;

                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }

                // Perspective-correct depth interpolation.
                let inv_z =
                    w0 / a.z + w1 / b.z + w2 / c.z;

                if inv_z <= 0.0 {
                    continue;
                }

                let depth = 1.0 / inv_z;
                let index = y * self.width + x;

                if depth < self.depth[index] {
                    self.depth[index] = depth;
                    let _ = gpu.pixel(x, y, color);
                }
            }
        }
    }

    pub fn render_cube(
        &mut self,
        gpu: &mut Gpu,
        yaw: f32,
        pitch: f32,
    ) {
        self.depth.fill(f32::INFINITY);

        gpu.clear([6, 13, 12, 255]);

        let vertices = [
            Vertex { x: -1.0, y: -1.0, z: -1.0 },
            Vertex { x:  1.0, y: -1.0, z: -1.0 },
            Vertex { x:  1.0, y:  1.0, z: -1.0 },
            Vertex { x: -1.0, y:  1.0, z: -1.0 },
            Vertex { x: -1.0, y: -1.0, z:  1.0 },
            Vertex { x:  1.0, y: -1.0, z:  1.0 },
            Vertex { x:  1.0, y:  1.0, z:  1.0 },
            Vertex { x: -1.0, y:  1.0, z:  1.0 },
        ];

        let projected: Vec<Vertex> = vertices
            .iter()
            .map(|v| self.project(Self::rotate(*v, yaw, pitch)))
            .collect();

        let faces: [([usize; 4], [u8; 4]); 6] = [
            ([0, 1, 2, 3], [0, 220, 80, 255]),
            ([5, 4, 7, 6], [0, 110, 240, 255]),
            ([4, 0, 3, 7], [255, 150, 20, 255]),
            ([1, 5, 6, 2], [220, 35, 80, 255]),
            ([3, 2, 6, 7], [180, 75, 245, 255]),
            ([4, 5, 1, 0], [35, 200, 210, 255]),
        ];

        for (indices, color) in faces {
            let a = projected[indices[0]];
            let b = projected[indices[1]];
            let c = projected[indices[2]];
            let d = projected[indices[3]];

            self.triangle(gpu, a, b, c, color);
            self.triangle(gpu, a, c, d, color);
        }

        gpu.present();
    }
}

pub fn render_cube_frame(
    width: usize,
    height: usize,
    yaw: f32,
    pitch: f32,
) -> Result<Vec<u8>, &'static str> {
    let mut gpu = Gpu::new(width, height)?;
    let mut renderer = Renderer3D::new(width, height);
    renderer.render_cube(&mut gpu, yaw, pitch);
    Ok(gpu.framebuffer().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cube_has_visible_pixels() {
        let pixels = render_cube_frame(
            640, 360, 0.4, 0.3
        ).unwrap();

        assert_eq!(pixels.len(), 640 * 360 * 4);

        assert!(pixels.chunks_exact(4).any(
            |p| p != [6, 13, 12, 255]
        ));
    }
}
