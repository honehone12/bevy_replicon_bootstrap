
fn setup_ball(mut commands: Commands) {
    let ball_1 = commands.spawn((
        Replicated,
        Culling::Disable,
        Ball::ServerSimulation,
        TransformBundle::from_transform(Transform{
            translation: BALL_POSITION_1,
            ..default()
        }),
        DynamicRigidBodyBundle::new(
            BALL_MASS, 
            Vec3::ZERO, 
            Vec3::ZERO
        ),
        NetworkTranslationBundle::<NetworkTranslation3D>::new(
            BALL_POSITION_1, 
            TranslationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        NetworkRotationBundle::<NetworkEuler>::new(
            Quat::IDENTITY, 
            RotationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        Collider::ball(BALL_RADIUS)
    ))
    .id();
    info!("ball 1: {ball_1:?} spawned");

    let ball_2 = commands.spawn((
        Replicated,
        Culling::Disable,
        Ball::ClientPrediction,
        TransformBundle::from_transform(Transform{
            translation: BALL_POSITION_2,
            ..default()
        }),
        DynamicRigidBodyBundle::new(
            BALL_MASS, 
            Vec3::ZERO, 
            Vec3::ZERO
        ),
        NetworkTranslationBundle::<NetworkTranslation3D>::new(
            BALL_POSITION_2, 
            TranslationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        NetworkRotationBundle::<NetworkEuler>::new(
            Quat::IDENTITY, 
            RotationAxis::Default, 
            0, 
            LARGE_CACHE_SIZE
        ).expect("sytem time looks earlier than unix epoch"),
        NetworkLinearVelocity3D::default(),
        NetworkAngularVelocity3D::default(),
        Collider::ball(BALL_RADIUS)
    ))
    .id();
    info!("ball 2: {ball_2:?} spwaned");
}

fn handle_ball_spawned(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    query: Query<(
        Entity,
        &Ball,
        &NetworkTranslation3D,
        &NetworkEuler,
        Option<&NetworkLinearVelocity3D>,
        Option<&NetworkAngularVelocity3D>,
        &ConfirmHistory
    ),
        Added<Ball>
    >,
    axis: Res<TransformAxis>
) {
    for (
        e, ball,
        net_trans, net_rot, 
        net_linvel, net_angvel,
        confirmed_tick
    ) in query.iter() {
        let material = match ball {
            Ball::ServerSimulation => materials.add(Color::from(BALL_COLOR_1)),
            Ball::ClientPrediction => materials.add(Color::from(BALL_COLOR_2)),
        };

        commands.entity(e)
        .insert(PbrBundle{
            mesh: meshes.add(Mesh::from(Sphere::new(BALL_RADIUS))),
            material,
            transform: Transform{
                translation: net_trans.to_vec3(axis.translation),
                rotation: net_rot.to_quat(axis.rotation),
                ..default()
            },
            ..default()
        });

        let tick = confirmed_tick.last_tick()
        .get();

        match ball {
            Ball::ServerSimulation => {
                commands.entity(e)
                .insert((
                    RigidBody::KinematicPositionBased,
                    ComponentCache::<NetworkTranslation3D>::with_init(
                        *net_trans,
                        tick, 
                        SMALL_CACHE_SIZE
                    ).expect("sytem time looks earlier than unix epoch"),
                    ComponentCache::<NetworkEuler>::with_init(
                        *net_rot, 
                        tick, 
                        SMALL_CACHE_SIZE
                    ).expect("sytem time looks earlier than unix epoch"),
                ))
            } 
            Ball::ClientPrediction => {
                commands.entity(e)
                .insert((
                    RigidBody::KinematicVelocityBased,
                    Velocity{
                        linvel: net_linvel.unwrap_or(&default()).0,
                        angvel: net_angvel.unwrap_or(&default()).0
                    }

                    // DynamicRigidBodyBundle::new(
                    //     BALL_MASS,
                    //     net_linvel.unwrap_or(&default()).0, 
                    //     net_angvel.unwrap_or(&default()).0
                    // ),
                ))
            }
        };

        commands.entity(e)
        .insert(Collider::ball(BALL_RADIUS));

        info!("ball: {e:?} spwaned");
    }
}
