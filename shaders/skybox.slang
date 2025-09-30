struct PushConstants {
    float4x4 view_proj;
    float3 sky_color;
    float3 sun_dir;
};

[[vk::push_constant]]
uniform PushConstants pushConstants;

static float3 skybox_vertices[8] = {
    float3(-1.0, -1.0, -1.0), // 0: v0
    float3( 1.0, -1.0, -1.0), // 1: v1
    float3( 1.0,  1.0, -1.0), // 2: v2
    float3(-1.0,  1.0, -1.0), // 3: v3
    float3(-1.0, -1.0,  1.0), // 4: v4
    float3( 1.0, -1.0,  1.0), // 5: v5
    float3( 1.0,  1.0,  1.0), // 6: v6
    float3(-1.0,  1.0,  1.0), // 7: v7
};

struct VSOutput {
    float4 position : SV_Position;
};

[shader("vertex")]
VSOutput vs_main(uint gl_VertexIndex : SV_VertexID) {
    VSOutput result;

    float3 vertex_pos = skybox_vertices[gl_VertexIndex];

    float4 temp_pos = mul(pushConstants.view_proj, float4(vertex_pos, 1.0));

    result.position = float4(temp_pos.xyz, temp_pos.w);
    result.position.z = result.position.w;

    return result;
}

[shader("fragment")]
float4 fs_main(float4 position: SV_Position) : SV_Target {	
    float3 view_dir = normalize(position.xyz);

    float sun_intensity = dot(view_dir, pushConstants.sun_dir);

    float sun_disk = smoothstep(0.99, 1.0, sun_intensity);

    float3 final_color = lerp(pushConstants.sky_color, float3(0.9, 0.8, 0.7), sun_disk);

    return float4(final_color, 1.0);
}
