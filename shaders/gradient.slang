// Slang version 1.0

// Push constant structure using 32-bit unsigned integers
// This avoids the need for Int8/Int16 shader capabilities
struct PushConstants {
    // timeOfDay: u16 (lower 16 bits)
    // yaw:       u16 (upper 16 bits)
    uint timeOfDayYaw;
    // pitch:     u8 (lowest 8 bits of this uint)
    // The rest of this uint serves as padding to ensure 4-byte alignment
    // for the entire push constant block as required by Vulkan.
    uint pitchAndPadding;
};

[[vk::push_constant]]
uniform PushConstants pushConstants;

[[vk::binding(0)]] RWTexture2D<float4> image;


// Define the local workgroup size
[shader("compute")]
[numthreads(16, 16, 1)]
void main(
    uint3 globalId : SV_DispatchThreadID
)
{
    // Output image declaration

    int2 texelCoord = globalId.xy;
    
    // Get image dimensions
    uint width, height;
    image.GetDimensions(width, height);
    int2 size = int2(width, height);

    if(texelCoord.x < size.x && texelCoord.y < size.y)
    {
        // Normalize coordinates for sky calculation
        float2 uv = float2(texelCoord) / float2(size); // uv from 0 to 1

        // Extract values from packed uints
        uint timeOfDay = pushConstants.timeOfDayYaw & 0xFFFF;           // Get lower 16 bits
        uint yaw = (pushConstants.timeOfDayYaw >> 16) & 0xFFFF;         // Get upper 16 bits
        uint pitch = pushConstants.pitchAndPadding & 0xFF;              // Get lowest 8 bits

        // float normalizedTimeOfDay = float(timeOfDay) / 65535.0;
        // float normalizedYaw = float(yaw) / 360.0;
        // float normalizedPitch = float(pitch) / 180.0;
        //
        // float4 color = float4(0.0, 0.0, 0.0, 1.0);
        //
        // color.r = normalizedTimeOfDay;
        // color.g = normalizedYaw;
        // color.b = normalizedPitch;
        // color.a = 1.0;
        //
        // image.Store(texelCoord, color);

        // 1. Process Time of Day
        float normalizedTime = float(timeOfDay) / 65535.0;

        float4 skyColor;
        float4 nightColor = float4(0.05, 0.05, 0.15, 1.0);
        float4 dawnDuskColor = float4(0.8, 0.4, 0.1, 1.0);
        float4 dayColor = float4(0.4, 0.6, 0.9, 1.0);

        if (normalizedTime < 0.25) {
            skyColor = lerp(nightColor, dawnDuskColor, normalizedTime / 0.25);
        } else if (normalizedTime < 0.5) {
            skyColor = lerp(dawnDuskColor, dayColor, (normalizedTime - 0.25) / 0.25);
        } else if (normalizedTime < 0.75) {
            skyColor = lerp(dayColor, dawnDuskColor, (normalizedTime - 0.5) / 0.25);
        } else {
            skyColor = lerp(dawnDuskColor, nightColor, (normalizedTime - 0.75) / 0.25);
        }

        float brightnessMultiplier = 1.0;
        if (normalizedTime < 0.2 || normalizedTime > 0.8) {
            brightnessMultiplier = 0.2;
        } else if (normalizedTime < 0.3 || normalizedTime > 0.7) {
            brightnessMultiplier = 0.5;
        }
        skyColor.rgb *= brightnessMultiplier;


        // 2. Process Pitch (Vertical Gradient)
        float normalizedPitchDegrees = float(pitch) / 255.0 * 180.0;
        float horizonBlend = uv.y;
        float pitchOffset = (normalizedPitchDegrees - 90.0) / 180.0;
        horizonBlend = saturate(uv.y + pitchOffset);
        float gradientFactor = smoothstep(0.0, 1.0, 1.0 - horizonBlend);
        skyColor.rgb *= (0.5 + 0.5 * gradientFactor);


        // 3. Process Yaw (Subtle Horizontal Tint/Cloud Effect)
        float normalizedYaw = float(yaw) / 65535.0;
        float cloudiness = sin(uv.x * 3.14159 * 5.0 + normalizedYaw * 10.0) * 0.1 + 0.5;
        skyColor.rgb = lerp(skyColor.rgb, skyColor.rgb * float3(0.9, 0.95, 1.0) * cloudiness, 0.1);

        // Final color output
        image.Store(texelCoord, skyColor);
    }
}
