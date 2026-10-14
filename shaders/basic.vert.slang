struct VertexInput
{
    float x : TEXCOORD0;
    float y : TEXCOORD1;
    float z : TEXCOORD2;
}

struct VertexOutput
{
    float4 Position : SV_Position;
};

[shader("vertex")]
VertexOutput main(VertexInput input)
{
    VertexOutput output;
    output.Position = float4(input.x, input.y, input.z, 1.0f);

    return output;
}
