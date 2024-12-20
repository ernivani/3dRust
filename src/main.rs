use macroquad::prelude::*;

const MOVE_SPEED: f32 = 0.1;
const LOOK_SPEED: f32 = 0.1;

#[macroquad::main("VoxelEngine")]
async fn main() {
    let mut camera = Camera3D {
        position: vec3(4.0, 4.0, 4.0),
        target: vec3(0.0, 0.0, 0.0),
        up: vec3(0.0, 1.0, 0.0),
        fovy: 45.0,
        aspect: Some(screen_width() / screen_height()),
        render_target: None,
        viewport: Some((0, 0, screen_width() as i32, screen_height() as i32)),
        projection: Projection::Perspective,
    };

    let mut captured = false;
    let mut last_mouse_position: Option<Vec2> = None;

    loop {
        // Toggle mouse capture with Escape
        if is_key_pressed(KeyCode::Escape) {
            captured = !captured;
            set_cursor_grab(captured);
            show_mouse(!captured);
            last_mouse_position = None;
        }

        // Camera controls
        if is_key_down(KeyCode::W) {
            let forward = (camera.target - camera.position).normalize();
            camera.position += forward * MOVE_SPEED;
            camera.target += forward * MOVE_SPEED;
        }
        if is_key_down(KeyCode::S) {
            let backward = (camera.position - camera.target).normalize();
            camera.position += backward * MOVE_SPEED;
            camera.target += backward * MOVE_SPEED;
        }
        if is_key_down(KeyCode::A) {
            let right = (camera.target - camera.position).cross(camera.up).normalize();
            camera.position -= right * MOVE_SPEED;
            camera.target -= right * MOVE_SPEED;
        }
        if is_key_down(KeyCode::D) {
            let right = (camera.target - camera.position).cross(camera.up).normalize();
            camera.position += right * MOVE_SPEED;
            camera.target += right * MOVE_SPEED;
        }

        // Mouse look
        if captured {
            let current_pos = Vec2::new(mouse_position().0, mouse_position().1);
            
            if let Some(last_pos) = last_mouse_position {
                let delta = (current_pos - last_pos) * 0.003;  // Reduced sensitivity
                
                if delta.length() > 0.0 {
                    let right = (camera.target - camera.position).cross(camera.up).normalize();
                    let rotation_y = Quat::from_rotation_y(-delta.x * LOOK_SPEED);
                    let rotation_x = Quat::from_axis_angle(right, -delta.y * LOOK_SPEED);
                    let rotation = rotation_y * rotation_x;
                    
                    let view_dir = (camera.target - camera.position).normalize();
                    let rotated_dir = rotation * view_dir;
                    camera.target = camera.position + rotated_dir;
                }
            }
            
            last_mouse_position = Some(current_pos);
        }

        clear_background(SKYBLUE);
        set_camera(&camera);

        // Draw grid
        draw_grid(20, 1.0, BLACK, GRAY);

        // Draw a sample voxel (cube)
        draw_cube(Vec3::ZERO, vec3(1.0, 1.0, 1.0), None, GREEN);

        // Reset to default camera
        set_default_camera();

        // Draw FPS counter
        let fps_text = format!("FPS: {}", get_fps());
        let text_dims = measure_text(&fps_text, None, 20, 1.0);
        draw_text(&fps_text, screen_width() - text_dims.width - 10.0, 20.0, 20.0, WHITE);

        draw_text(
            if !captured {
                "Appuyez sur ESC pour capturer la souris et commencer à la regarder"
            } else {
                "ZQSD pour se déplacer, Mouse pour regarder, ESC pour relacher la souris"
            },
            10.0, 20.0, 20.0, BLACK
        );

        next_frame().await
    }
}
