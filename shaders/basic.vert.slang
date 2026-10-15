struct VertexInput
{
    [[vk::location(0)]]
    float x : TEXCOORD0;
    [[vk::location(1)]]
    float y : TEXCOORD1;
    [[vk::location(2)]]
    float z : TEXCOORD2;
    [[vk::location(3)]]
    float3 color : COLOR;
}

struct VertexOutput
{
    float4 Position : SV_Position;

    [[vk::location(0)]]
    float3 color : COLOR;
};

[shader("vertex")]
VertexOutput main(VertexInput input)
{
    VertexOutput output;
    output.Position = float4(input.x, input.y, input.z, 1.0f);

    // Hand the per-vertex color to the rasterizer, which interpolates it
    // across the triangle before the fragment shader sees it.
    output.color = input.color;

    return output;
}
