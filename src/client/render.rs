//! Block-entity drawing (J/client/renderer/CuttingBoardRenderer.java).
use mod_api::*;

/// `Direction.toYRot()` of `facing.getOpposite()`.
pub fn board_y_rot(facing: &str) -> f32 {
    // opposite: north→south(0), south→north(180), west→east(270), east→west(90)
    match facing {
        "north" => 0.0,
        "south" => 180.0,
        "west" => 270.0,
        _ => 90.0,
    }
}

/// `CuttingBoardRenderer#getModelCount`.
pub fn model_count(count: u32, max_stack: u32) -> u32 {
    if count > 1 {
        1 + ((count as f32 / max_stack.max(1) as f32) * 4.0).ceil() as u32
    } else {
        1
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Pose {
    Flat,
    Block,
    Carved { angle: f32 },
}

/// Draw the board's stack as the reference does.
pub fn draw_board(
    draw: &mut dyn BlockEntityDrawContext,
    stack: &ItemStack,
    facing: &str,
    pose: Pose,
    layers: u32,
    seed: i32,
) {
    let y_rot = board_y_rot(facing);
    let mut random = LegacyRandom::new(seed as i64);
    for i in 0..layers {
        let (dx, dz) = if layers == 1 {
            (0.0, 0.0)
        } else {
            (
                (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5,
                (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5,
            )
        };
        draw.push();
        match pose {
            Pose::Carved { angle } => {
                draw.translate([0.5, 0.23, 0.5]);
                draw.rotate_about([0.0, 1.0, 0.0], -y_rot + 180.0, [0.0; 3]);
                draw.rotate_about([0.0, 0.0, 1.0], angle, [0.0; 3]);
                draw.scale([0.6; 3]);
            }
            Pose::Block => {
                draw.translate([0.5 + dx, 0.27 + 0.03 * (i + 1) as f32, 0.5 + dz]);
                draw.rotate_about([0.0, 1.0, 0.0], -y_rot, [0.0; 3]);
                draw.scale([0.8; 3]);
            }
            Pose::Flat => {
                draw.translate([0.5 + dx, 0.08 + 0.03 * (i + 1) as f32, 0.5 + dz]);
                draw.rotate_about([0.0, 1.0, 0.0], -y_rot, [0.0; 3]);
                draw.rotate_about([1.0, 0.0, 0.0], 90.0, [0.0; 3]);
                draw.scale([0.6; 3]);
            }
        }
        draw.item(stack, ItemDisplayContext::Fixed, None, 0);
        draw.pop();
    }
}

/// `SkilletRenderer#submit`: the stack lies flat in the pan, layered, rotated by facing.
pub fn draw_skillet(
    draw: &mut dyn BlockEntityDrawContext,
    stack: &ItemStack,
    facing: &str,
    layers: u32,
    seed: i32,
) {
    // direction = FACING (not its opposite); toYRot: south 0, west 90, north 180, east 270
    let y_rot = match facing {
        "south" => 0.0,
        "west" => 90.0,
        "north" => 180.0,
        _ => 270.0,
    };
    let mut random = LegacyRandom::new(seed as i64);
    for i in 0..layers {
        let dx = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
        let dz = (random.next_float() * 2.0 - 1.0) * 0.15 * 0.5;
        draw.push();
        draw.translate([0.5 + dx, 0.1 + 0.03 * (i + 1) as f32, 0.5 + dz]);
        draw.rotate_about([0.0, 1.0, 0.0], -y_rot, [0.0; 3]);
        draw.rotate_about([1.0, 0.0, 0.0], 90.0, [0.0; 3]);
        draw.scale([0.5; 3]);
        draw.item(stack, ItemDisplayContext::Fixed, None, 0);
        draw.pop();
    }
}

/// `DefaultStoveRenderer#getStoveItemOffset`.
pub const STOVE_OFFSETS: [[f32; 2]; 6] = [
    [0.3, 0.2],
    [0.0, 0.2],
    [-0.3, 0.2],
    [0.3, -0.2],
    [0.0, -0.2],
    [-0.3, -0.2],
];

/// `AbstractStoveRenderer#submit`: each stack flat on top, rotated by facing, at its slot offset.
pub fn draw_stove(
    draw: &mut dyn BlockEntityDrawContext,
    stacks: &[Option<ItemStack>],
    facing: &str,
) {
    let y_rot = board_y_rot(facing);
    for (i, stack) in stacks.iter().enumerate().take(STOVE_OFFSETS.len()) {
        let Some(stack) = stack else { continue };
        let [ox, oy] = STOVE_OFFSETS[i];
        draw.push();
        draw.translate([0.5, 1.02, 0.5]);
        draw.rotate_about([0.0, 1.0, 0.0], -y_rot, [0.0; 3]);
        draw.rotate_about([1.0, 0.0, 0.0], 90.0, [0.0; 3]);
        draw.translate([ox, oy, 0.0]);
        draw.scale([0.375; 3]);
        draw.item(stack, ItemDisplayContext::Fixed, None, 0);
        draw.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_counts() {
        assert_eq!(model_count(1, 64), 1);
        assert_eq!(model_count(2, 64), 2);
        assert_eq!(model_count(16, 64), 2);
        assert_eq!(model_count(17, 64), 3);
        assert_eq!(model_count(64, 64), 5);
    }
}
