//! UI do cliente — controles de zoom (feature-plugin).

use bevy::picking::events::PointerClick;
use bevy::prelude::*;
use bevy::text::FontSize;

use crate::CamOrbit;

/// Registra a UI do cliente.
pub(crate) fn plugin(app: &mut App) {
    app.add_systems(Startup, setup_ui);
}

/// UI de zoom no topo-direito: label "Zoom" + botões "+" e "-". Cada botão ajusta a
/// distância da câmera orbital (`CamOrbit::dist`) via observer nativo de clique.
fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Text::new("Zoom"),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::WHITE),
            ));
            // (+) aproxima (dist menor); (-) afasta (dist maior).
            for (label, factor) in [("+", 0.85_f32), ("-", 1.18_f32)] {
                root.spawn((
                    Node {
                        width: Val::Px(36.0),
                        height: Val::Px(36.0),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.18, 0.18, 0.22)),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(label),
                        TextFont { font_size: FontSize::Px(24.0), ..default() },
                        TextColor(Color::WHITE),
                    ));
                })
                .observe(move |_: On<PointerClick>, mut orbit: ResMut<CamOrbit>| {
                    orbit.zoom = (orbit.zoom * factor).clamp(6.0, 40.0);
                });
            }
        });
}
