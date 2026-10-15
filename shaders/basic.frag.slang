struct FragmentInput
{
    [[vk::location(0)]]
    float3 color : COLOR;
}

struct FragmentOutput
{
    float4 Color : SV_Target0;
};

[shader("fragment")]
FragmentOutput main(FragmentInput input)
{
    FragmentOutput output;

    // The color arriving here is the blend of the triangle's three corner
    // colors at this pixel, produced by the rasterizer's interpolation.
    output.Color = float4(input.color, 1.0f);

    return output;
}
