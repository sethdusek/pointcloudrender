# Point Cloud Render
WGPU-based renderer for images + depth maps that allows you to view the model from arbitrary positions. Example:

|Image|Depth Map|
|-----|---------|
|![image-resized](https://github.com/user-attachments/assets/02d21429-3b2e-487a-a523-2b4fcf1bd3a4)|![depth-resized](https://github.com/user-attachments/assets/683705ab-9de5-40c6-a338-51b3aca7cd57)|

    cargo run --release image.jpg depth.jpg

![image](https://github.com/user-attachments/assets/2779c66a-4c3d-45ff-8773-76aa1802a952)

## Keybindings
|Key|Purpose|
|---|-------|
|w/a/s/d/q/e|Rotate image|
|f|Take screenshot|
|t|Toggle background shading (on by default). Fills in holes in image at the cost of performance|
|y|Toggle occlusion shading (on by default). Fills in holes by replacing them with pixels from nearby occluding pixels|
|[ ]| Increase/Decreasee background shading iterations|
| ; '| Increase/Decrease occlusion shading iterations|
