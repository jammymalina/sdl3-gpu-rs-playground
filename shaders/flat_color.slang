import common_types;

ConstantBuffer<SceneData> scene : register(b0);
ConstantBuffer<ObjectData> object : register(b1);
ConstantBuffer<BasicColorData> color : register(b2);

struct VertexInput
{
    [vk::location(0)]
    float3 position : POSITION;
    [vk::location(1)]
    float3 normal : NORMAL;
};

struct VertexOutput
{
    float4 position : SV_Position;
    nointerpolation float4 color : COLOR;
};

[shader("vertex")]
VertexOutput vertex_main(VertexInput input)
{
    VertexOutput output;
    output.position = mul(mul(scene.view_projection, object.model), float4(input.position, 1.0f));

    float4 world_normal = mul(object.normal, float4(input.normal, 0.0f));
    float n_dot_l = max(dot(world_normal, scene.light_direction), 0.0f);
    output.color = color.base_color * n_dot_l + color.ambient_color;

    return output;
}

[shader("fragment")]
float4 frag_main(VertexOutput input) : SV_Target
{
    return input.color;
}

