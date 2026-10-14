struct FragmentInput
{
}

struct FragmentOutput
{
    float4 Color : SV_Target0;
};

[shader("fragment")]
FragmentOutput main(FragmentInput input)
{
    FragmentOutput output;
    output.Color = float4(1.0f, 0.0f, 0.0f, 1.0f); // Solid Red (R=1, G=0, B=0, A=1)

    return output;
}
