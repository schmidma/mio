use std::{
    collections::HashMap,
    f32::consts::{FRAC_PI_2, PI},
};

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_inspector_egui_rapier::InspectableRapierPlugin;
use bevy_rapier3d::{prelude::*, rapier::prelude::ColliderBuilder};
use color_eyre::{eyre::WrapErr, Result};
use field_dimensions::FieldDimensions;
use inspector_ui::{InspectorSettings, InspectorUiPlugin};
use pan_orbit_camera::{pan_orbit_camera, PanOrbitCamera};
use urdf_rs::{JointType, Robot};

mod field_dimensions;
mod inspector_ui;
mod pan_orbit_camera;

fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin {
            mode: DebugRenderMode::COLLIDER_SHAPES | DebugRenderMode::JOINTS,
            //| DebugRenderMode::RIGID_BODY_AXES,
            ..Default::default()
        })
        .add_plugin(EguiPlugin)
        .add_plugin(DefaultInspectorConfigPlugin)
        .add_plugin(InspectorUiPlugin)
        .add_plugin(InspectableRapierPlugin)
        .insert_resource(InspectorSettings { enabled: true })
        .insert_resource(RobotSpecification {
            urdf: urdf_rs::read_file("assets/NAO.urdf")
                .wrap_err("Failed to load urdf specification for NAO")?,
        })
        .insert_resource(RapierConfiguration {
            gravity: Vec3::NEG_Z,
            ..Default::default()
        })
        .insert_resource(FieldDimensions::default())
        .add_startup_system(setup_camera)
        .add_startup_system(setup_field)
        .add_startup_system(setup_robot)
        .add_system(pan_orbit_camera)
        .run();
    Ok(())
}

#[derive(Component)]
struct MainCamera;

fn setup_camera(mut commands: Commands) {
    let initial_camera_position = Vec3::new(0.0, -10.0, 4.0);
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(initial_camera_position)
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..Default::default()
        })
        .insert(PanOrbitCamera {
            radius: initial_camera_position.length(),
            ..Default::default()
        })
        .insert(MainCamera);
}

fn setup_field(
    mut commands: Commands,
    field_dimensions: Res<FieldDimensions>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let ground_size = Vec2::new(
        field_dimensions.length + field_dimensions.border_strip_width * 2.0,
        field_dimensions.width + field_dimensions.border_strip_width * 2.0,
    );
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::new(ground_size))),
            material: materials.add(StandardMaterial {
                base_color: Color::GREEN,
                perceptual_roughness: 0.99,
                ..Default::default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, -1.0),
            ..Default::default()
        })
        .insert(Collider::cuboid(
            ground_size.x / 2.0,
            ground_size.y / 2.0,
            0.01,
        ))
        .insert(CollisionGroups::new(Group::GROUP_1, Group::ALL))
        .insert(Name::new("field"));

    commands
        .spawn(RigidBody::Dynamic)
        .insert(Name::new("ball"))
        .insert(Collider::ball(field_dimensions.ball_radius))
        .insert(CollisionGroups::new(Group::GROUP_3, Group::ALL))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 4.0)));

    const HALF_SIZE: f32 = 10.0;
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..default()
            },
            shadows_enabled: true,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 10.0),
            rotation: Quat::from_rotation_x(-PI / 4.0),
            ..default()
        },
        ..default()
    });
}

#[derive(Resource)]
struct RobotSpecification {
    urdf: Robot,
}

fn setup_robot(mut commands: Commands, robot_specification: Res<RobotSpecification>) {
    let mut links = HashMap::new();

    for link in &robot_specification.urdf.links {
        // let origin = link.inertial.origin.xyz;
        // let origin = Vec3::new(origin[0] as f32, origin[1] as f32, origin[2] as f32);
        // let roll_pitch_yaw = link.inertial.origin.rpy;
        // let roll_pitch_yaw = Quat::from_euler(
        //     EulerRot::XYZ,
        //     roll_pitch_yaw[0] as f32,
        //     roll_pitch_yaw[1] as f32,
        //     roll_pitch_yaw[2] as f32,
        // );

        let mut entity = commands.spawn(Name::new(link.name.clone()));
        entity
            .insert(RigidBody::Dynamic)
            .insert(TransformBundle::default());

        if !link.collision.is_empty() {
            let shapes = link
                .collision
                .iter()
                .map(|collision| {
                    let collider = match &collision.geometry {
                        urdf_rs::Geometry::Box { size } => Collider::cuboid(
                            size[0] as f32 / 2.0,
                            size[1] as f32 / 2.0,
                            size[2] as f32 / 2.0,
                        ),
                        urdf_rs::Geometry::Cylinder { radius, length } => {
                            Collider::cylinder(*length as f32 / 2.0, *radius as f32)
                        }
                        urdf_rs::Geometry::Sphere { radius } => Collider::ball(*radius as f32),
                        _ => todo!(),
                    };
                    let position = collision.origin.xyz;
                    let position =
                        Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);
                    let rotation = collision.origin.rpy;
                    let rotation = Quat::from_euler(
                        EulerRot::ZYX,
                        rotation[2] as f32,
                        rotation[1] as f32,
                        rotation[0] as f32,
                    );
                    (position, rotation, collider)
                })
                .collect();
            entity
                .insert(Collider::compound(shapes))
                .insert(CollisionGroups::new(
                    Group::GROUP_2,
                    Group::GROUP_1 | Group::GROUP_3,
                ));
        }
        links.insert(link.name.clone(), entity.id());
    }

    for joint in robot_specification.urdf.joints.iter() {
        let parent_id = links[&joint.parent.link];
        let child_id = links[&joint.child.link];
        commands.entity(parent_id).add_child(child_id);
        let translation = joint.origin.xyz;
        let translation = Vec3::new(
            translation[0] as f32,
            translation[1] as f32,
            translation[2] as f32,
        );
        let rotation = joint.origin.rpy;
        let rotation = Quat::from_euler(
            EulerRot::ZYX,
            rotation[2] as f32,
            rotation[1] as f32,
            rotation[0] as f32,
        );
        let axis = joint.axis.xyz;
        let axis = Vec3::new(axis[0] as f32, axis[1] as f32, axis[2] as f32);
        let mut child = commands.entity(child_id);
        child.insert(Transform {
            translation,
            rotation,
            ..Default::default()
        });
        // let joint = FixedJointBuilder::new()
        //     .local_anchor1(translation)
        //     .local_basis1(rotation);
        // child.insert(ImpulseJoint::new(parent_id, joint));
        match joint.joint_type {
            JointType::Revolute => {
                let joint = RevoluteJointBuilder::new(axis).local_anchor1(translation);
                child.insert(ImpulseJoint::new(parent_id, joint));
            }
            JointType::Continuous => (),
            JointType::Prismatic => {
                let joint = PrismaticJointBuilder::new(axis).local_anchor1(translation);
                child.insert(ImpulseJoint::new(parent_id, joint));
            }
            JointType::Fixed => {
                let joint = FixedJointBuilder::new()
                    .local_anchor1(translation)
                    .local_basis1(rotation);
                child.insert(ImpulseJoint::new(parent_id, joint));
            }
            JointType::Floating => {
                todo!();
            }
            JointType::Planar => {
                todo!();
            }
            JointType::Spherical => {
                let joint = SphericalJointBuilder::new().local_anchor1(translation);
                child.insert(ImpulseJoint::new(parent_id, joint));
            }
        };
    }
}
