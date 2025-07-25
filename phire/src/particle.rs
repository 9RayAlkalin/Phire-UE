// This is from https://raw.githubusercontent.com/not-fl3/macroquad/master/particles/src/lib.rs
// We can't use macroquad-particles directly, since it implicitly depends on quad-snd (which is
// obviously a mistake) and that conflicts with kira.

// Edits:
// 1. nanoserde related parts are removed for simplicity's sake.
// 2. apply_viewport
// 3. clippy
// 4. time can be customized by input argument
// 5. Remove EmittersCache

use macroquad::prelude::*;
use macroquad::window::miniquad::*;
use rand_pcg::rand_core::RngCore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    Bezier,
}

#[derive(Debug, Clone)]
pub struct Curve {
    /// Key points for building a curve
    pub points: Vec<(f32, f32)>,
    /// The way middle points is interpolated during building a curve
    /// Only Linear is implemented now
    pub interpolation: Interpolation,
    /// Interpolation steps used to build the curve from the key points
    pub resolution: usize,
}

impl Curve {
    fn batch(&self) -> BatchedCurve {
        if self.interpolation == Interpolation::Bezier {
            unimplemented!()
        }

        let step_f32 = 1.0 / self.resolution as f32;
        let mut x = 0.0;
        let mut points = Vec::with_capacity(self.resolution);

        for curve_part in self.points.windows(2) {
            let start = curve_part[0];
            let end = curve_part[1];

            while x <= end.0 {
                let t = (x - start.0) / (end.0 - start.0);
                let point = start.1 + (end.1 - start.1) * t;
                points.push(point);
                x += step_f32;
            }
        }

        BatchedCurve { points }
    }
}

#[derive(Debug, Clone)]
pub struct BatchedCurve {
    pub points: Vec<f32>,
}

impl BatchedCurve {
    fn get(&self, t: f32) -> f32 {
        let t_scaled = t * self.points.len() as f32;
        let previous_ix = (t_scaled as usize).min(self.points.len() - 1);
        let next_ix = (previous_ix + 1).min(self.points.len() - 1);
        let previous = self.points[previous_ix];
        let next = self.points[next_ix];

        previous + (next - previous) * (t_scaled - previous_ix as f32)
    }
}
impl Default for Curve {
    fn default() -> Curve {
        Curve {
            points: vec![],
            interpolation: Interpolation::Linear,
            resolution: 20,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum EmissionShape {
    Point,
    Rect { width: f32, height: f32 },
    Sphere { radius: f32 },
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ColorCurve {
    pub start: Color,
    pub mid: Color,
    pub end: Color,
}

impl Default for ColorCurve {
    fn default() -> ColorCurve {
        ColorCurve {
            start: WHITE,
            mid: WHITE,
            end: WHITE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EmitterConfig {
    pub max_particles: usize,
    pub rng: Option<rand_pcg::Lcg64Xsh32>,
    /// If false - particles spawns at position supplied to .draw(), but afterwards lives in current camera coordinate system.
    /// If false particles use coordinate system originated to the emitter draw position
    pub local_coords: bool,
    /// Particles will be emitted inside that region.
    pub emission_shape: EmissionShape,
    /// If true only one emission cycle occurs. May be re-emitted by .emit() call.
    pub one_shot: bool,
    /// Lifespan of individual particle.
    pub lifetime: f32,
    /// Particle lifetime randomness ratio.
    /// Each particle will spawned with "lifetime = lifetime - lifetime * rand::gen_range(0.0, lifetime_randomness)".
    pub lifetime_randomness: f32,
    /// 0..1 value, how rapidly particles in emission cycle are emitted.
    /// With 0 particles will be emitted with equal gap.
    /// With 1 all the particles will be emitted at the beginning of the cycle.
    pub explosiveness: f32,
    /// Amount of particles emitted in one emission cycle.
    pub amount: u32,
    /// Shape of each individual particle mesh.
    pub shape: ParticleShape,
    /// Particles are emitting when "emitting" is true.
    /// If its a "one-shot" emitter, emitting will switch to false after active emission cycle.
    pub emitting: bool,
    /// Unit vector specifying emission direction.
    pub initial_direction: Vec2,
    /// Angle from 0 to "2 * Pi" for random fluctuation for direction vector.
    pub initial_direction_spread: f32,
    /// Initial speed for each emitted particle.
    /// Direction of the initial speed vector is affected by "direction" and "spread"
    pub initial_velocity: f32,
    /// Initial velocity randomness ratio.
    /// Each particle will spawned with "initial_velocity = initial_velocity - initial_velocity * rand::gen_range(0.0, initial_velocity_randomness)".
    pub initial_velocity_randomness: f32,
    /// Velocity acceleration applied to each particle in the direction of motion.
    pub linear_accel: f32,

    // Initial rotation for each emitted particle.
    pub initial_rotation: f32,
    /// Initial rotation randomness.
    /// Each particle will spawned with "initial_rotation = initial_rotation - initial_rotation * rand::gen_range(0.0, initial_rotation_randomness)".
    pub initial_rotation_randomness: f32,
    // Initial rotational speed
    pub initial_angular_velocity: f32,
    /// Initial angular velocity randomness.
    /// Each particle will spawned with "initial_angular_velocity = initial_angular_velocity - initial_angular_velocity * rand::gen_range(0.0, initial_angular_velocity_randomness)".
    pub initial_angular_velocity_randomness: f32,
    /// Angular velocity acceleration applied to each particle .
    pub angular_accel: f32,
    /// Angluar velocity damping
    /// Each frame angular velocity will be transformed "angular_velocity *= (1.0 - angular_damping)".
    pub angular_damping: f32,
    /// Each particle is a "size x size" square.
    pub size: f32,
    /// Each particle will spawned with "size = size - size * rand::gen_range(0.0, size_randomness)".
    pub size_randomness: f32,
    /// If curve is present in each moment of particle lifetime size would be multiplied by the value from the curve
    pub size_curve: Option<Curve>,

    /// Particles rendering mode.
    pub blend_mode: BlendMode,

    pub base_color: Color,
    /// How particles should change base color along the lifetime.
    pub colors_curve: ColorCurve,

    /// Gravity applied to each individual particle.
    pub gravity: Vec2,

    /// Particle texture. If none particles going to be white squares.
    pub texture: Option<Texture2D>,

    /// For animated texture specify spritesheet layout.
    /// If none the whole texture will be used.
    pub atlas: Option<AtlasConfig>,

    /// Custom material used to shade each particle.
    pub material: Option<ParticleMaterial>,

    /// If none particles will be rendered directly to the screen.
    /// If not none all the particles will be rendered to a rectangle and than this rectangle
    /// will be rendered to the screen.
    /// This will allows some effects affecting particles as a whole.
    /// NOTE: this is not really implemented and now Some will just make hardcoded downscaling
    pub post_processing: Option<PostProcessing>,
}

impl EmissionShape {
    fn gen_random_point(&self) -> Vec2 {
        match self {
            EmissionShape::Point => vec2(0., 0.),
            EmissionShape::Rect { width, height } => vec2(rand::gen_range(-width / 2., width / 2.0), rand::gen_range(-height / 2., height / 2.0)),
            EmissionShape::Sphere { radius } => {
                let ro = rand::gen_range(0., radius * radius).sqrt();
                let phi = rand::gen_range(0., std::f32::consts::PI * 2.);

                macroquad::math::polar_to_cartesian(ro, phi)
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PostProcessing;

#[derive(Clone, Debug, PartialEq)]
pub enum ParticleShape {
    Rectangle { aspect_ratio: f32 },
    Circle { subdivisions: u32 },
    CustomMesh { vertices: Vec<f32>, indices: Vec<u16> },
}

impl ParticleShape {
    fn build_bindings(&self, ctx: &mut miniquad::Context, positions_vertex_buffer: Buffer, texture: Option<Texture2D>) -> Bindings {
        let (geometry_vertex_buffer, index_buffer) = match self {
            ParticleShape::Rectangle { aspect_ratio } => {
                #[rustfmt::skip]
                let vertices: &[f32] = &[
                    // positions          uv          colors
                    -1.0 * aspect_ratio, -1.0, 0.0,   0.0, 0.0,  1.0, 1.0, 1.0, 1.0,
                     1.0 * aspect_ratio, -1.0, 0.0,   1.0, 0.0,  1.0, 1.0, 1.0, 1.0,
                     1.0 * aspect_ratio,  1.0, 0.0,   1.0, 1.0,  1.0, 1.0, 1.0, 1.0,
                    -1.0 * aspect_ratio,  1.0, 0.0,   0.0, 1.0,  1.0, 1.0, 1.0, 1.0,
                ];

                let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, vertices);

                #[rustfmt::skip]
                let indices: &[u16] = &[
                    0, 1, 2, 0, 2, 3
                ];
                let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, indices);

                (vertex_buffer, index_buffer)
            }
            ParticleShape::Circle { subdivisions } => {
                let mut vertices = Vec::<f32>::new();
                let mut indices = Vec::<u16>::new();

                let rot = 0.0;
                vertices.extend_from_slice(&[0., 0., 0., 0., 0., 1.0, 1.0, 1.0, 1.0]);
                for i in 0..subdivisions + 1 {
                    let rx = (i as f32 / *subdivisions as f32 * std::f32::consts::PI * 2. + rot).cos();
                    let ry = (i as f32 / *subdivisions as f32 * std::f32::consts::PI * 2. + rot).sin();
                    vertices.extend_from_slice(&[rx, ry, 0., rx, ry, 1., 1., 1., 1.]);

                    if i != *subdivisions {
                        indices.extend_from_slice(&[0, i as u16 + 1, i as u16 + 2]);
                    }
                }

                let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, &vertices);
                let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, &indices);
                (vertex_buffer, index_buffer)
            }
            ParticleShape::CustomMesh { vertices, indices } => {
                let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, vertices);
                let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, indices);
                (vertex_buffer, index_buffer)
            }
        };

        Bindings {
            vertex_buffers: vec![geometry_vertex_buffer, positions_vertex_buffer],
            index_buffer,
            images: vec![
                texture.map_or_else(|| Texture::from_rgba8(ctx, 1, 1, &[255, 255, 255, 255]), |texture| texture.raw_miniquad_texture_handle())
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParticleMaterial {
    vertex: String,
    fragment: String,
}

impl ParticleMaterial {
    pub fn new(vertex: &str, fragment: &str) -> ParticleMaterial {
        ParticleMaterial {
            vertex: vertex.to_owned(),
            fragment: fragment.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Colors of overlapped particles will be blended by alpha channel.
    Alpha,
    /// Colors of overlapped particles will be added to each other.
    Additive,
}

impl BlendMode {
    fn blend_state(&self) -> BlendState {
        match self {
            BlendMode::Alpha => {
                BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::OneMinusValue(BlendValue::SourceAlpha))
            }
            BlendMode::Additive => BlendState::new(Equation::Add, BlendFactor::Value(BlendValue::SourceAlpha), BlendFactor::One),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AtlasConfig {
    n: u16,
    m: u16,
    start_index: u16,
    end_index: u16,
}

impl AtlasConfig {
    pub fn new<T: std::ops::RangeBounds<u16>>(n: u16, m: u16, range: T) -> AtlasConfig {
        let start_index = match range.start_bound() {
            std::ops::Bound::Unbounded => 0,
            std::ops::Bound::Included(i) => *i,
            std::ops::Bound::Excluded(i) => i + 1,
        };
        let end_index = match range.end_bound() {
            std::ops::Bound::Unbounded => n * m,
            std::ops::Bound::Included(i) => i - 1,
            std::ops::Bound::Excluded(i) => *i,
        };

        AtlasConfig {
            n,
            m,
            start_index,
            end_index,
        }
    }
}

impl Default for EmitterConfig {
    fn default() -> EmitterConfig {
        EmitterConfig {
            max_particles: 20000,
            rng: None,
            local_coords: false,
            emission_shape: EmissionShape::Point,
            one_shot: false,
            lifetime: 1.0,
            lifetime_randomness: 0.0,
            amount: 8,
            shape: ParticleShape::Rectangle { aspect_ratio: 1.0 },
            explosiveness: 0.0,
            emitting: true,
            initial_direction: vec2(0., -1.),
            initial_direction_spread: 0.,
            initial_velocity: 50.0,
            initial_velocity_randomness: 0.0,
            linear_accel: 0.0,
            initial_rotation: 0.0,
            initial_rotation_randomness: 0.0,
            initial_angular_velocity: 0.0,
            initial_angular_velocity_randomness: 0.0,
            angular_accel: 0.0,
            angular_damping: 0.0,
            size: 10.0,
            size_randomness: 0.0,
            size_curve: None,
            blend_mode: BlendMode::Alpha,
            base_color: WHITE,
            colors_curve: ColorCurve::default(),
            gravity: vec2(0.0, 0.0),
            texture: None,
            atlas: None,
            material: None,
            post_processing: None,
        }
    }
}

#[repr(C)]
struct GpuParticle {
    pos: Vec4,
    uv: Vec4,
    data: Vec4,
    color: Vec4,
}

struct CpuParticle {
    velocity: Vec2,
    angular_velocity: f32,
    lived: f32,
    lifetime: f32,
    frame: u16,
    initial_size: f32,
    color: Color,
}

pub struct Emitter {
    pipeline: Pipeline,
    bindings: Bindings,
    post_processing_pass: RenderPass,
    post_processing_pipeline: Pipeline,
    post_processing_bindings: Bindings,

    gpu_particles: Vec<GpuParticle>,
    cpu_counterpart: Vec<CpuParticle>,

    last_emit_time: f32,
    time_passed: f32,

    particles_spawned: u64,
    position: Vec2,

    batched_size_curve: Option<BatchedCurve>,

    blend_mode: BlendMode,
    mesh_dirty: bool,

    pub config: EmitterConfig,
}

impl Emitter {
    pub fn new(config: EmitterConfig) -> Emitter {
        let InternalGlContext { quad_context: ctx, .. } = unsafe { get_internal_gl() };

        // empty, dynamic instance-data vertex buffer
        let positions_vertex_buffer = Buffer::stream(ctx, BufferType::VertexBuffer, config.max_particles * std::mem::size_of::<GpuParticle>());

        let bindings = config.shape.build_bindings(ctx, positions_vertex_buffer, config.texture);

        let (vertex, fragment) = config
            .material
            .as_ref()
            .map_or_else(|| (shader::VERTEX, shader::FRAGMENT), |material| (&material.vertex, &material.fragment));

        let shader = {
            use macroquad::material::shaders::{preprocess_shader, PreprocessorConfig};

            let config = PreprocessorConfig {
                includes: vec![("particles.glsl".to_string(), include_str!("particles.glsl").to_owned())],
            };

            let vertex = preprocess_shader(vertex, &config);
            let fragment = preprocess_shader(fragment, &config);

            Shader::new(ctx, &vertex, &fragment, shader::meta()).unwrap()
        };

        let blend_mode = config.blend_mode.blend_state();
        let pipeline = Pipeline::with_params(
            ctx,
            &[
                BufferLayout::default(),
                BufferLayout {
                    step_func: VertexStep::PerInstance,
                    ..Default::default()
                },
            ],
            &[
                VertexAttribute::with_buffer("in_attr_pos", VertexFormat::Float3, 0),
                VertexAttribute::with_buffer("in_attr_uv", VertexFormat::Float2, 0),
                VertexAttribute::with_buffer("in_attr_color", VertexFormat::Float4, 0),
                VertexAttribute::with_buffer("in_attr_inst_pos", VertexFormat::Float4, 1),
                VertexAttribute::with_buffer("in_attr_inst_uv", VertexFormat::Float4, 1),
                VertexAttribute::with_buffer("in_attr_inst_data", VertexFormat::Float4, 1),
                VertexAttribute::with_buffer("in_attr_inst_color", VertexFormat::Float4, 1),
            ],
            shader,
            PipelineParams {
                color_blend: Some(blend_mode),
                alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::Zero, BlendFactor::One)),
                ..Default::default()
            },
        );

        let post_processing_shader =
            Shader::new(ctx, post_processing_shader::VERTEX, post_processing_shader::FRAGMENT, post_processing_shader::meta()).unwrap();

        let post_processing_pipeline = Pipeline::with_params(
            ctx,
            &[BufferLayout::default(), BufferLayout::default()],
            &[
                VertexAttribute::with_buffer("pos", VertexFormat::Float2, 0),
                VertexAttribute::with_buffer("uv", VertexFormat::Float2, 0),
            ],
            post_processing_shader,
            PipelineParams {
                color_blend: Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                )),
                ..Default::default()
            },
        );
        let post_processing_pass = {
            let color_img = Texture::new_render_texture(
                ctx,
                TextureParams {
                    width: 320,
                    height: 200,
                    format: TextureFormat::RGBA8,
                    ..Default::default()
                },
            );
            color_img.set_filter(ctx, FilterMode::Nearest);

            RenderPass::new(ctx, color_img, None)
        };

        let post_processing_bindings = {
            #[rustfmt::skip]
            let vertices: &[f32] = &[
                // positions   uv
                -1.0, -1.0,    0.0, 0.0,
                 1.0, -1.0,    1.0, 0.0,
                 1.0,  1.0,    1.0, 1.0,
                -1.0,  1.0,    0.0, 1.0,
            ];

            let vertex_buffer = Buffer::immutable(ctx, BufferType::VertexBuffer, vertices);

            #[rustfmt::skip]
            let indices: &[u16] = &[
                0, 1, 2, 0, 2, 3
            ];
            let index_buffer = Buffer::immutable(ctx, BufferType::IndexBuffer, indices);
            Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                images: vec![post_processing_pass.texture(ctx)],
            }
        };
        let max_particles = config.max_particles;

        Emitter {
            blend_mode: config.blend_mode,
            batched_size_curve: config.size_curve.as_ref().map(|curve| curve.batch()),
            post_processing_pass,
            post_processing_pipeline,
            post_processing_bindings,
            config,
            pipeline,
            bindings,
            position: vec2(0.0, 0.0),
            gpu_particles: Vec::with_capacity(max_particles),
            cpu_counterpart: Vec::with_capacity(max_particles),
            particles_spawned: 0,
            last_emit_time: 0.0,
            time_passed: 0.0,
            mesh_dirty: false,
        }
    }

    pub fn rebuild_size_curve(&mut self) {
        self.batched_size_curve = self.config.size_curve.as_ref().map(|curve| curve.batch());
    }

    pub fn update_particle_mesh(&mut self) {
        self.mesh_dirty = true;
    }

    fn emit_particle(&mut self, offset: Vec2) {
        if self.gpu_particles.len() == self.config.max_particles {
            return;
        }
        let offset = offset + self.config.emission_shape.gen_random_point();

        fn map_u64_to_f32(r: u64, min: f32, max: f32) -> f32 {
            let frac = (r as f64) / (u64::MAX as f64 + 1.0);
            min + (max - min) * (frac as f32)
        }

        let mut random_initial_vector = |dir: Vec2, spread: f32, velocity: f32| -> Vec2 {
            let angle = if let Some(rng) = self.config.rng.as_mut() {
                map_u64_to_f32(rng.next_u64(), -spread / 2.0, spread / 2.0)
            } else {
                rand::gen_range(-spread / 2.0, spread / 2.0)
            };

            let quat = glam::Quat::from_rotation_z(angle);
            let dir = quat * vec3(dir.x, dir.y, 0.0);
            let res = dir * velocity;

            vec2(res.x, res.y)
        };

        let r = self.config.size - self.config.size * rand::gen_range(0.0, self.config.size_randomness);

        let rotation = self.config.initial_rotation - self.config.initial_rotation * rand::gen_range(0.0, self.config.initial_rotation_randomness);

        let particle = if self.config.local_coords {
            GpuParticle {
                pos: vec4(offset.x, offset.y, rotation, r),
                uv: vec4(1.0, 1.0, 0.0, 0.0),
                data: vec4(self.particles_spawned as f32, 0.0, 0.0, 0.0),
                color: self.config.colors_curve.start.to_vec(),
            }
        } else {
            GpuParticle {
                pos: vec4(self.position.x + offset.x, self.position.y + offset.y, rotation, r),
                uv: vec4(1.0, 1.0, 0.0, 0.0),
                data: vec4(self.particles_spawned as f32, 0.0, 0.0, 0.0),
                color: self.config.colors_curve.start.to_vec(),
            }
        };

        self.particles_spawned += 1;
        self.gpu_particles.push(particle);
        self.cpu_counterpart.push(CpuParticle {
            velocity: random_initial_vector(
                vec2(self.config.initial_direction.x, self.config.initial_direction.y),
                self.config.initial_direction_spread,
                self.config.initial_velocity - self.config.initial_velocity * rand::gen_range(0.0, self.config.initial_velocity_randomness),
            ),
            angular_velocity: self.config.initial_angular_velocity
                - self.config.initial_angular_velocity * rand::gen_range(0.0, self.config.initial_angular_velocity_randomness),
            lived: 0.0,
            lifetime: self.config.lifetime - self.config.lifetime * rand::gen_range(0.0, self.config.lifetime_randomness),
            frame: 0,
            initial_size: r,
            color: self.config.base_color,
        });
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        if self.mesh_dirty {
            self.bindings = self
                .config
                .shape
                .build_bindings(ctx, self.bindings.vertex_buffers[1], self.config.texture);
            self.mesh_dirty = false;
        }
        if self.config.emitting {
            self.time_passed += dt;

            let gap = (self.config.lifetime / self.config.amount as f32) * (1.0 - self.config.explosiveness);

            let spawn_amount = if gap < 0.001 {
                // to prevent division by 0 problems
                self.config.amount as usize
            } else {
                // how many particles fits into this delta time
                ((self.time_passed - self.last_emit_time) / gap) as usize
            };

            for _ in 0..spawn_amount {
                self.last_emit_time = self.time_passed;

                if self.particles_spawned < self.config.amount as u64 {
                    self.emit_particle(vec2(0.0, 0.0));
                }

                if self.gpu_particles.len() >= self.config.amount as usize {
                    break;
                }
            }
        }

        if self.config.one_shot && self.time_passed > self.config.lifetime {
            self.time_passed = 0.0;
            self.last_emit_time = 0.0;
            self.config.emitting = false;
        }

        for (gpu, cpu) in self.gpu_particles.iter_mut().zip(&mut self.cpu_counterpart) {
            // TODO: this is not quite the way to apply acceleration, this is not
            // fps independent and just wrong
            cpu.velocity += cpu.velocity * self.config.linear_accel * dt;
            cpu.angular_velocity += cpu.angular_velocity * self.config.angular_accel * dt;
            cpu.angular_velocity *= 1.0 - self.config.angular_damping;

            gpu.color = {
                let t = cpu.lived / cpu.lifetime;
                if t < 0.5 {
                    let t = t * 2.;
                    self.config.colors_curve.start.to_vec() * (1.0 - t) + self.config.colors_curve.mid.to_vec() * t
                } else {
                    let t = (t - 0.5) * 2.;
                    self.config.colors_curve.mid.to_vec() * (1.0 - t) + self.config.colors_curve.end.to_vec() * t
                }
            };
            gpu.color *= cpu.color.to_vec();
            gpu.pos += vec4(cpu.velocity.x, cpu.velocity.y, cpu.angular_velocity, 0.0) * dt;

            gpu.pos.w = cpu.initial_size * self.batched_size_curve.as_ref().map_or(1.0, |curve| curve.get(cpu.lived / cpu.lifetime));

            if cpu.lifetime != 0.0 {
                gpu.data.y = cpu.lived / cpu.lifetime;
            }

            //cpu.lived = f32::min(cpu.lived + dt, cpu.lifetime);
            cpu.lived += dt;
            cpu.velocity += self.config.gravity * dt;

            if let Some(atlas) = &self.config.atlas {
                if cpu.lifetime != 0.0 {
                    cpu.frame = (cpu.lived / cpu.lifetime * (atlas.end_index - atlas.start_index) as f32) as u16 + atlas.start_index;
                }

                let x = cpu.frame % atlas.n;
                let y = cpu.frame / atlas.n;

                gpu.uv = vec4(x as f32 / atlas.n as f32, y as f32 / atlas.m as f32, 1.0 / atlas.n as f32, 1.0 / atlas.m as f32);
            } else {
                gpu.uv = vec4(0.0, 0.0, 1.0, 1.0);
            }
        }

        for i in (0..self.gpu_particles.len()).rev() {
            // second if clause is just for the case when lifetime was changed in the editor
            // normally particle lifetime is always less or equal config lifetime
            if self.cpu_counterpart[i].lived >= self.cpu_counterpart[i].lifetime || self.cpu_counterpart[i].lived > self.config.lifetime {
                if self.cpu_counterpart[i].lived != self.cpu_counterpart[i].lifetime {
                    self.particles_spawned -= 1;
                }
                self.gpu_particles.swap_remove(i);
                self.cpu_counterpart.swap_remove(i);
            }
        }

        self.bindings.vertex_buffers[1].update(ctx, &self.gpu_particles[..]);
    }

    /// Immediately emit N particles, ignoring "emitting" and "amount" params of EmitterConfig
    pub fn emit(&mut self, pos: Vec2, n: usize) {
        for _ in 0..n {
            self.emit_particle(pos);
            self.particles_spawned += 1;
        }
    }

    fn perform_render_pass(&mut self, quad_gl: &QuadGl, ctx: &mut Context) {
        ctx.apply_bindings(&self.bindings);
        ctx.apply_uniforms(&shader::Uniforms {
            mvp: quad_gl.get_projection_matrix(),
            emitter_position: vec3(self.position.x, self.position.y, 0.0),
            local_coords: if self.config.local_coords { 1.0 } else { 0.0 },
        });

        ctx.draw(0, self.bindings.index_buffer.size() as i32 / std::mem::size_of::<u16>() as i32, self.gpu_particles.len() as i32);
    }

    pub fn setup_render_pass(&mut self, quad_gl: &QuadGl, ctx: &mut Context) {
        if self.config.blend_mode != self.blend_mode {
            self.pipeline.set_blend(ctx, Some(self.config.blend_mode.blend_state()));
            self.blend_mode = self.config.blend_mode;
        }

        if self.config.post_processing.is_none() {
            let pass = quad_gl.get_active_render_pass();
            if let Some(pass) = pass {
                ctx.begin_pass(pass, PassAction::Nothing);
            } else {
                ctx.begin_default_pass(PassAction::Nothing);
            }
        } else {
            ctx.begin_pass(self.post_processing_pass, PassAction::clear_color(0.0, 0.0, 0.0, 0.0));
        };

        ctx.apply_pipeline(&self.pipeline);
        // This is made
        let (x, y, w, h) = quad_gl
            .get_viewport()
            .unwrap_or_else(|| (0, 0, screen_width() as _, screen_height() as _));
        ctx.apply_viewport(x, y, w, h);
    }

    pub fn end_render_pass(&mut self, quad_gl: &QuadGl, ctx: &mut Context) {
        ctx.end_render_pass();

        if self.config.post_processing.is_some() {
            let pass = quad_gl.get_active_render_pass();
            if let Some(pass) = pass {
                ctx.begin_pass(pass, PassAction::Nothing);
            } else {
                ctx.begin_default_pass(PassAction::Nothing);
            }

            ctx.apply_pipeline(&self.post_processing_pipeline);
            let (x, y, w, h) = quad_gl
                .get_viewport()
                .unwrap_or_else(|| (0, 0, screen_width() as _, screen_height() as _));
            ctx.apply_viewport(x, y, w, h);

            ctx.apply_bindings(&self.post_processing_bindings);

            ctx.draw(0, 6, 1);

            ctx.end_render_pass();
        }
    }

    pub fn draw(&mut self, pos: Vec2, dt: f32) {
        let mut gl = unsafe { get_internal_gl() };

        gl.flush();

        let InternalGlContext { quad_context: ctx, quad_gl } = gl;

        self.position = pos;

        self.update(ctx, dt);

        self.setup_render_pass(quad_gl, ctx);
        self.perform_render_pass(quad_gl, ctx);
        self.end_render_pass(quad_gl, ctx);
    }
}

mod shader {
    use super::*;

    pub const VERTEX: &str = r#"#version 100
    #define DEF_VERTEX_ATTRIBUTES
    #include "particles.glsl"

    varying lowp vec2 texcoord;
    varying lowp vec4 color;

    void main() {
        gl_Position = particle_transform_vertex();
        color = in_attr_inst_color;
        texcoord = particle_transform_uv();
    }
    "#;

    pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec2 texcoord;
    varying lowp vec4 color;

    uniform sampler2D texture;

    void main() {
        gl_FragColor = texture2D(texture, texcoord) * color;
    }
    "#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["texture".to_string()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("_mvp", UniformType::Mat4),
                    UniformDesc::new("_local_coords", UniformType::Float1),
                    UniformDesc::new("_emitter_position", UniformType::Float3),
                ],
            },
        }
    }

    #[repr(C)]
    pub struct Uniforms {
        pub mvp: Mat4,
        pub local_coords: f32,
        pub emitter_position: Vec3,
    }
}

mod post_processing_shader {
    use super::*;

    pub const VERTEX: &str = r#"#version 100
    attribute vec2 pos;
    attribute vec2 uv;

    varying lowp vec2 texcoord;

    void main() {
        gl_Position = vec4(pos, 0, 1);
        texcoord = uv;
    }
    "#;

    pub const FRAGMENT: &str = r#"#version 100
    precision lowp float;

    varying vec2 texcoord;
    uniform sampler2D tex;

    void main() {
        gl_FragColor = texture2D(tex, texcoord);
    }
    "#;

    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["tex".to_string()],
            uniforms: UniformBlockLayout { uniforms: vec![] },
        }
    }
}
