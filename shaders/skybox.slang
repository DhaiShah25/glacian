struct PushConstants {
    float3 sky_color;
};

[[vk::push_constant]]
uniform PushConstants pushConstants;

static const float2 skybox_vertices[3] = {
    float2(-1.0, -1.0),
    float2( 3.0, -1.0),
    float2(-1.0,  3.0),
};

[shader("vertex")]
float4 vs_main(uint vert_idx : SV_VertexID) : SV_Position {
    float2 vertex_pos = skybox_vertices[vert_idx];
    return float4(vertex_pos, 1.0, 1.0);
}

[shader("fragment")]
float4 fs_main(float4 screenPos : SV_POSITION) : SV_Target {
    return float4(pushConstants.sky_color, 1.0);
}
