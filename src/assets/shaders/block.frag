#version 330 core
in vec2 TexCoord;
in float Position;
in float TextureIndex;
in float TextSize;

out vec4 FragColor;

uniform sampler2D grassTopTexture;    // texture unit 0
uniform sampler2D grassSideTexture;   // texture unit 1
uniform sampler2D dirtTexture;        // texture unit 2
uniform sampler2D colormapTexture;    // texture unit 3
uniform sampler2D grassSideOverlayTexture;  // texture unit 4
uniform sampler2D stoneTexture;       // texture unit 5
uniform sampler2D waterTexture;       // texture unit 6

void main() {
    vec4 biomeColor = texture(colormapTexture, vec2(0.5, 0.5));  // For now using center of colormap

    if (TextureIndex < 0.5) {  // grass top
        vec4 grassColor = texture(grassTopTexture, TexCoord);
        FragColor = grassColor * biomeColor;
    } else if (TextureIndex < 1.5) {  // grass side
        vec4 sideTexture = texture(grassSideTexture, TexCoord);
        vec4 overlayTexture = texture(grassSideOverlayTexture, TexCoord);
        
        // Apply the biome color to the overlay and blend it with the side texture
        vec4 coloredOverlay = overlayTexture * biomeColor;
        FragColor = mix(sideTexture, coloredOverlay, overlayTexture.a);
    } else if (TextureIndex < 2.5) {  // dirt
        FragColor = texture(dirtTexture, TexCoord);
    } else if (TextureIndex < 3.5) {  // stone
        FragColor = texture(stoneTexture, TexCoord);
    } else {  // water
        vec4 waterColor = texture(waterTexture, TexCoord);
        // Apply a blue tint to the water
        vec4 waterTint = vec4(0.0, 0.3, 0.8, 1.0);  // Blue color with some green for a natural look
        waterColor = waterColor * waterTint;
        waterColor.a = 0.6;  // Make water transparent
        FragColor = waterColor;
    }
} 