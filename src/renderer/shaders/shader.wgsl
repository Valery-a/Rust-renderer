// validate shader cargo run --features wgsl-in -- ../../xp-vox-engine/src/renderer/shaders/light_shader.wgsl

struct DirectionalLight
{
    direction: vec4<f32>;
    ambient: vec4<f32>;
    diffuse: vec4<f32>;
    specular: vec4<f32>;
};

struct SpotLight
{
    position: vec4<f32>;
    direction: vec4<f32>;
    ambient: vec4<f32>;
    diffuse: vec4<f32>;
    specular: vec4<f32>;
    cons: f32;
    linear: f32;
    quadratic: f32;
    cut_off_inner: f32;
    cut_off_outer: f32;
    p0: f32; p1: f32; p2: f32;
};

struct PointLight
{
    position: vec4<f32>;
    ambient: vec4<f32>;
    diffuse: vec4<f32>;
    specular: vec4<f32>;
    cons: f32;
    linear: f32;
    quadratic: f32;
    p0: f32;
};

[[block]]
struct Globals {
    view: mat4x4<f32>;
    proj: mat4x4<f32>;
    world_camera_position: vec4<f32>;
    material_specular: vec4<f32>;
    material_shininess: f32;
    nr_of_directional_lights: u32;
    nr_of_spot_lights: u32;
    nr_of_point_lights: u32;
};

[[group(0), binding(0)]]
var<uniform> u_globals: Globals;

[[block]]
struct DirectionalLights {
    lights: array<DirectionalLight, 1>;
};

[[group(0), binding(1)]]
var<uniform> directional_lights: DirectionalLights;

[[block]]
struct SpotLights {
    lights: array<SpotLight, 10>;
};

[[group(0), binding(2)]]
var<uniform> spot_lights: SpotLights;

[[block]]
struct PointLights {
    lights: array<PointLight, 10>;
};

[[group(0), binding(3)]]
var<uniform> point_lights: PointLights;

struct Instance {
    model: mat4x4<f32>;
    inverse_model: mat4x4<f32>;
};

[[block]]
struct Instances {
    models: array<Instance>;
};

[[group(0), binding(4)]]
var<storage> models: [[access(read)]] Instances;

struct VertexOutput {
    [[builtin(position)]] proj_position: vec4<f32>;
    [[location(0)]] world_position: vec3<f32>;
    [[location(1)]] world_normal: vec3<f32>;
    [[location(2)]] color: vec3<f32>;
};

[[stage(vertex)]]
fn vs_main([[builtin(instance_index)]] instance_idx: u32, [[location(0)]] model_position: vec3<f32>,
           [[location(1)]] model_normal: vec3<f32>,
           [[location(2)]] color: vec3<f32>) -> VertexOutput {
    let view = u_globals.view;
    let proj = u_globals.proj;
    let model = models.models[instance_idx].model;
    let inverse_transpose = transpose(models.models[instance_idx].inverse_model);
    var out: VertexOutput;
    out.proj_position = proj * view * model * vec4<f32>(model_position, 1.0);
    out.world_position = (model * vec4<f32>(model_position, 1.0)).xyz;
    out.world_normal = (inverse_transpose * vec4<f32>(model_normal, 1.0)).xyz;
    out.color = color;
    return out;
}

fn calculate_directional_light(normal: vec3<f32>, view_direction: vec3<f32>, light: DirectionalLight, material_specular: vec3<f32>, material_shininess: f32, in_color: vec3<f32>) -> vec3<f32>
{
    // negate light direction -> we want direction towards light
    let light_direction = normalize(-light.direction.xyz);
    // diffuse
    let diff = max(dot(normal, light_direction), 0.0);
    // specular
    let halfway_direction = normalize(light_direction + view_direction);
    let spec = pow(max(dot(view_direction, halfway_direction), 0.0), material_shininess);

    let ambient  = light.ambient.xyz * in_color;
    let diffuse = light.diffuse.xyz * diff * in_color;
    let specular = light.specular.xyz * spec * material_specular.xyz;

    return ambient + diffuse + specular;
}

fn calculate_spot_light(normal: vec3<f32>, view_direction: vec3<f32>, frag_position: vec3<f32>, light: SpotLight,  material_specular: vec3<f32>, material_shininess: f32, in_color: vec3<f32>) -> vec3<f32>
{
    let light_direction = normalize(light.position.xyz - frag_position);

    // diffuse
    let diff = max(dot(normal, light_direction), 0.0);
    // specular
    let halfway_direction = normalize(light_direction + view_direction);
    let spec = pow(max(dot(view_direction, halfway_direction), 0.0), material_shininess);
    // attenuation
    let len = distance(light.position.xyz, frag_position);
    let attenuation = 1.0 / (light.cons + light.linear * len + light.quadratic * len * len);
    // spotlight intensity
    let theta = dot(light_direction, normalize(-light.direction.xyz));
    let epsilon = light.cut_off_inner - light.cut_off_outer;
    let intensity = clamp((theta - light.cut_off_outer) / epsilon, 0.0, 1.0);
    let ambient = light.ambient.xyz * in_color * attenuation * intensity;
    let diffuse = light.diffuse.xyz * diff * in_color * attenuation * intensity;
    let specular = light.specular.xyz * spec * material_specular.xyz * attenuation * intensity;
    return ambient + diffuse + specular;
}

fn calculate_point_light(normal: vec3<f32>, view_direction: vec3<f32>, frag_position: vec3<f32>, light: SpotLight,  material_specular: vec3<f32>, material_shininess: f32, in_color: vec3<f32>) -> vec3<f32> {
    let light_direction = normalize(light.position.xyz - frag_position);

    // diffuse
    let diff = max(dot(normal, light_direction), 0.0);
    // specular
    let halfway_direction = normalize(light_direction + view_direction);
    let spec = pow(max(dot(view_direction, halfway_direction), 0.0), material_shininess);
    // attenuation
    let len = distance(light.position.xyz, frag_position);
    let attenuation = 1.0 / (light.cons + light.linear * len + light.quadratic * len * len);

    let ambient = light.ambient.xyz * in_color * attenuation;
    let diffuse = light.diffuse.xyz * diff * in_color * attenuation;
    let specular = light.specular.xyz * spec * material_specular.xyz * attenuation;
    return ambient + diffuse + specular;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let normal = normalize(in.world_normal);
    let view_direction = normalize(u_globals.world_camera_position.xyz - in.world_position);

    var result: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

    for(var i: u32 = 0u; i < u_globals.nr_of_directional_lights; i = i + 1u) {
        result = result + calculate_directional_light(normal, view_direction, directional_lights.lights[i], u_globals.material_specular.xyz, u_globals.material_shininess, in.color);
    }

    for(var i: u32 = 0u; i < u_globals.nr_of_spot_lights; i = i + 1u) {
        result = result + calculate_spot_light(normal, view_direction, in.world_position, spot_lights.lights[i], u_globals.material_specular.xyz, u_globals.material_shininess, in.color);
    }

    for(var i: u32 = 0u; i < u_globals.nr_of_point_lights; i = i + 1u) {
        result = result + calculate_point_light(normal, view_direction, in.world_position, spot_lights.lights[i], u_globals.material_specular.xyz, u_globals.material_shininess, in.color);
    }

    let gamma: f32 = 2.2;
    let color = vec4<f32>(pow(result, vec3<f32>(1.0 / gamma)), 1.0);
    return color;
}