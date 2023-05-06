use std::{collections::HashMap, f32::consts::PI};

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
// use bevy_inspector_egui_rapier::InspectableRapierPlugin;
use bevy_rapier3d::prelude::*;
use bevy_stl::StlPlugin;
use color_eyre::{eyre::WrapErr, Result};
use field_dimensions::FieldDimensions;
// use inspector_ui::{InspectorSettings, InspectorUiPlugin};

use smooth_bevy_cameras::{
    controllers::orbit::{OrbitCameraBundle, OrbitCameraController, OrbitCameraPlugin},
    LookTransformPlugin,
};
use urdf_rs::{JointType, Robot};

mod field_dimensions;
mod inspector_ui;

fn main() -> Result<()> {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin {
            mode: DebugRenderMode::COLLIDER_SHAPES | DebugRenderMode::JOINTS,
            //| DebugRenderMode::RIGID_BODY_AXES,
            ..Default::default()
        })
        .add_plugin(StlPlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(DefaultInspectorConfigPlugin)
        //.add_plugin(InspectorUiPlugin)
        //.add_plugin(InspectableRapierPlugin)
        //.insert_resource(InspectorSettings { enabled: true })
        .insert_resource(RobotSpecification {
            urdf: urdf_rs::read_file("assets/NAO.urdf")
                .wrap_err("Failed to load urdf specification for NAO")?,
        })
        .insert_resource(RapierConfiguration {
            gravity: Vec3::NEG_Z,
            ..Default::default()
        })
        .insert_resource(FieldDimensions::default())
        .add_plugin(LookTransformPlugin)
        .add_plugin(OrbitCameraPlugin::default())
        .add_startup_system(spawn_camera)
        .add_startup_system(setup_field)
        .add_startup_systems(
            (
                setup_links,
                apply_system_buffers,
                setup_joints,
                add_link_visuals,
            )
                .chain(),
        )
        .run();
    Ok(())
}

#[derive(Component)]
struct MainCamera;

fn spawn_camera(mut commands: Commands) {
    commands
        .spawn(Camera3dBundle::default())
        .insert(OrbitCameraBundle::new(
            OrbitCameraController::default(),
            Vec3::new(-2.0, 5.0, 5.0),
            Vec3::new(0., 0., 0.),
            Vec3::Y,
        ));
}

fn setup_field(
    mut commands: Commands,
    field_dimensions: Res<FieldDimensions>,
    server: Res<AssetServer>,
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
                base_color: Color::rgb(0.0, 0.2, 0.0),
                perceptual_roughness: 0.8,
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
        .insert(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: field_dimensions.ball_radius,
                sectors: 30,
                stacks: 30,
            })),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: Some(server.load("ball/football_base_color.jpg")),
                metallic: 0.,
                perceptual_roughness: 0.4,
                normal_map_texture: Some(server.load("ball/football_normal.jpg")),
                ..Default::default()
            }),
            ..Default::default()
        })
        .insert(Collider::ball(field_dimensions.ball_radius))
        .insert(CollisionGroups::new(Group::GROUP_3, Group::ALL))
        .insert(Restitution::coefficient(0.7))
        .insert(TransformBundle::from(Transform::from_xyz(0.0, 0.0, 4.0)));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
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

#[derive(Component)]
struct NaoRobot;

#[derive(Component)]
struct NaoLink {
    pub name: String,
}

fn add_link_visuals(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    robot_specification: Res<RobotSpecification>,
    links: Query<(Entity, &NaoLink)>,
) {
    let mut link_to_entity = HashMap::new();
    for (entity, nao_link) in links.iter() {
        link_to_entity.insert(&nao_link.name, entity);
    }

    for link in robot_specification.urdf.links.iter() {
        let current_link = link_to_entity[&link.name];

        if !link.visual.is_empty() {
            link.visual.iter().for_each(|visual| {
                let (mesh, scale): (Handle<Mesh>, _) = match &visual.geometry {
                    urdf_rs::Geometry::Mesh { filename, scale } => (
                        server.load(filename),
                        scale
                            .map(|vec| Vec3::new(vec[0] as f32, vec[1] as f32, vec[2] as f32))
                            .unwrap_or(Vec3::ONE),
                    ),
                    _ => (Default::default(), Vec3::ONE),
                };
                let material: Handle<StandardMaterial> = match &visual.material {
                    Some(urdf_rs::Material {
                        texture: Some(urdf_rs::Texture { filename }),
                        ..
                    }) => server.load(filename),
                    Some(urdf_rs::Material {
                        color: Some(urdf_rs::Color { rgba }),
                        ..
                    }) => materials.add(
                        Color::rgba(
                            rgba.0[0] as f32,
                            rgba.0[1] as f32,
                            rgba.0[2] as f32,
                            rgba.0[3] as f32,
                        )
                        .into(),
                    ),
                    _ => materials.add(Color::rgb(0.3, 0.2, 0.8).into()),
                };

                let position = visual.origin.xyz;
                let rotation = visual.origin.rpy;

                let origin =
                    Transform::from_xyz(position[0] as f32, position[1] as f32, position[2] as f32)
                        .with_rotation(Quat::from_euler(
                            EulerRot::ZYX,
                            rotation[2] as f32,
                            rotation[1] as f32,
                            rotation[0] as f32,
                        ))
                        .with_scale(scale);

                let visual = commands
                    .spawn(PbrBundle {
                        mesh,
                        material,
                        transform: origin,
                        ..Default::default()
                    })
                    .id();
                commands.entity(current_link).add_child(visual);
            });
        }
    }
}

fn setup_joints(
    mut commands: Commands,
    robot_specification: Res<RobotSpecification>,
    links: Query<(Entity, &NaoLink)>,
) {
    let mut link_to_entity = HashMap::new();
    for (entity, nao_link) in links.iter() {
        link_to_entity.insert(&nao_link.name, entity);
    }

    for joint in robot_specification.urdf.joints.iter() {
        let parent_id = link_to_entity[&joint.parent.link];
        let child_id = link_to_entity[&joint.child.link];

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

fn setup_links(
    mut commands: Commands,
    server: Res<AssetServer>,
    robot_specification: Res<RobotSpecification>,
) {
    for link in &robot_specification.urdf.links {
        let name = link.name.clone();

        let shapes: Vec<_> = link
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
                    urdf_rs::Geometry::Capsule { radius, length } => {
                        Collider::capsule_z(*length as f32 / 2.0, *radius as f32)
                    }
                    urdf_rs::Geometry::Mesh { filename, .. } => {
                        let _mesh: Handle<Mesh> = server.load(filename);
                        todo!();
                    }
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

        let mut link = commands.spawn((
            NaoLink { name },
            RigidBody::Dynamic,
            TransformBundle::default(),
            VisibilityBundle::default(),
        ));
        if shapes.len() > 0 {
            link.insert(Collider::compound(shapes))
                .insert(CollisionGroups::new(
                    Group::GROUP_2,
                    Group::GROUP_1 | Group::GROUP_3,
                ));
        }
    }
}
