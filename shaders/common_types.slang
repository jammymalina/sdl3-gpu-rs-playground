module common_types;

public struct SceneData
{
    public float4x4 view_projection;
    public float3 camera_position;
    public float _padding0;
    public float4 light_direction;
    public float4 light_color;
};

public struct ObjectData
{
    public float4x4 model;
    public float4x4 normal;
};

public struct BasicColorData
{
    public float4 base_color;
    public float4 ambient_color;
}
