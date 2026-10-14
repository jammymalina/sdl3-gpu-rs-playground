struct VertexInput
{
    [[vk::location(0)]]
    float3 position : POSITION;
    [[vk::location(1)]]
    float3 color : COLOR;
}

struct VertexOutput
{
    float4 clip_position : SV_Position;
    [[vk::location(0)]]
    float3 color : COLOR;
}

struct Camera
{
    float4x4 view_projection;
}

[[vk::binding(0, 1)]]
ConstantBuffer<Camera> camera;

[shader("vertex")]
VertexOutput main(VertexInput input)
{
    VertexOutput output;
    output.clip_position = mul(camera.view_projection, float4(input.position, 1.0));
    output.color = input.color;
    return output;
}
