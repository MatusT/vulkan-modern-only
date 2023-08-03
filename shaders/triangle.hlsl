struct PushConstants
{
      uint64_t globals_address;
};

[[vk::push_constant]] PushConstants pushConstants;

// struct Globals {
//     float4x4 model;
// };

struct VertexInput
{
	uint vertexId: SV_VertexId;
};

struct VertexOutput
{
    float4 position: SV_POSITION;
};

VertexOutput vertexMain(VertexInput input)
{
    float4x4 model = vk::RawBufferLoad<float4x4>(pushConstants.globals_address);

    VertexOutput vertexOutput;
    
    float2 positions[3] = {
        float2(0.0, -0.5),
        float2(0.5, 0.5),
        float2(-0.5, 0.5)
    };

    vertexOutput.position = mul(float4(positions[input.vertexId], 0.0, 1.0), model);

    return vertexOutput;
}

struct FragmentOutput {
    float4 color: SV_TARGET;
};

FragmentOutput pixelMain(VertexOutput vertexOutput)
{
    FragmentOutput output;
    output.color = float4(1.0, 1.0, 1.0, 1.0);

    return output;
}