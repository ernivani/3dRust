#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;
layout (location = 2) in float aPosition;
layout (location = 3) in float aTextureIndex;
layout (location = 4) in float aTextSize;

uniform mat4 transform;

out vec2 TexCoord;
out float Position;
out float TextureIndex;
out float TextSize;

void main() {
    gl_Position = transform * vec4(aPos, 1.0);
    TexCoord = aTexCoord;
    Position = aPosition;
    TextureIndex = aTextureIndex;
    TextSize = aTextSize;
} 