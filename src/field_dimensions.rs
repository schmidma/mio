use bevy::prelude::*;

#[derive(Clone, Debug, Resource)]
pub struct FieldDimensions {
    pub ball_radius: f32,
    pub length: f32,
    pub width: f32,
    pub line_width: f32,
    pub penalty_marker_size: f32,
    pub goal_box_area_length: f32,
    pub goal_box_area_width: f32,
    pub penalty_area_length: f32,
    pub penalty_area_width: f32,
    pub penalty_marker_distance: f32,
    pub center_circle_diameter: f32,
    pub border_strip_width: f32,
    pub goal_inner_width: f32,
    pub goal_post_diameter: f32,
    pub goal_depth: f32,
}

impl Default for FieldDimensions {
    fn default() -> Self {
        Self {
            ball_radius: 0.05,
            length: 9.0,
            width: 6.0,
            line_width: 0.05,
            penalty_marker_size: 0.1,
            goal_box_area_length: 0.6,
            goal_box_area_width: 2.2,
            penalty_area_length: 1.65,
            penalty_area_width: 4.0,
            penalty_marker_distance: 1.3,
            center_circle_diameter: 1.5,
            border_strip_width: 0.7,
            goal_inner_width: 1.5,
            goal_post_diameter: 0.1,
            goal_depth: 0.5,
        }
    }
}
// "field_dimensions": {
//   "ball_radius": 0.05,
//   "length": 9.0,
//   "width": 6.0,
//   "line_width": 0.05,
//   "penalty_marker_size": 0.1,
//   "goal_box_area_length": 0.6,
//   "goal_box_area_width": 2.2,
//   "penalty_area_length": 1.65,
//   "penalty_area_width": 4.0,
//   "penalty_marker_distance": 1.3,
//   "center_circle_diameter": 1.5,
//   "border_strip_width": 0.7,
//   "goal_inner_width": 1.5,
//   "goal_post_diameter": 0.1,
//   "goal_depth": 0.5
// },
