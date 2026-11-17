//! Câmera orbital estilo Ragnarok — órbita, rotação livre e auto-rotação anti-oclusão
//! (feature-plugin).

use bevy::camera::ScalingMode;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use shared::world::{HOUSE_X0, HOUSE_X1, HOUSE_Y0, HOUSE_Y1};
use shared::{Player, Position};

use crate::vision::in_house;
use crate::{ClientSet, MyPlayer};

/// Registra a câmera orbital e seus sistemas nos sets `Input`/`Camera`.
pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<CamOrbit>().add_systems(
        Update,
        (
            rotate_input.in_set(ClientSet::Input),
            // auto_rotate_camera ajusta o yaw ANTES de camera_orbit posicionar a câmera —
            // preserva a sequência original (sem lag de 1 frame na auto-rotação).
            (auto_rotate_camera, camera_orbit).chain().in_set(ClientSet::Camera),
        ),
    );
}

/// Estado da câmera orbital estilo RO: `yaw` gira livre ao redor do personagem; `pitch` é
/// o tilt (olhar de cima); `zoom` = altura (em tiles) visível na tela — projeção
/// ORTOGRÁFICA (sem perspectiva, achatada como o RO). Menor `zoom` = mais perto.
#[derive(Resource)]
pub(crate) struct CamOrbit {
    pub(crate) yaw: f32,
    pitch: f32,
    pub(crate) zoom: f32,
    /// Sentido TRAVADO da auto-rotação anti-oclusão (0 = parada; ±1 = girando). Trava o
    /// sentido enquanto gira pra não "sambar" trocando de direção toda hora.
    auto_dir: f32,
}

impl Default for CamOrbit {
    fn default() -> Self {
        // pitch ~50° (olhar de cima estilo RO); ~14 tiles visíveis na vertical.
        Self { yaw: 0.6, pitch: 0.88, zoom: 14.0, auto_dir: 0.0 }
    }
}

/// Distância fixa da câmera ao foco (ortográfica: não afeta o tamanho, só o posicionamento
/// e o clipping — só precisa ser grande o bastante pra conter a cena).
const CAM_DIST: f32 = 90.0;

/// Gira a câmera livre ao redor do personagem (yaw): teclas Q/E **ou** scroll do mouse.
fn rotate_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut wheel: MessageReader<MouseWheel>,
    mut orbit: ResMut<CamOrbit>,
) {
    let speed = 1.6;
    if keys.pressed(KeyCode::KeyQ) {
        orbit.yaw -= speed * time.delta_secs();
    }
    if keys.pressed(KeyCode::KeyE) {
        orbit.yaw += speed * time.delta_secs();
    }
    // Scroll do mouse gira a câmera em torno do personagem (~10° por entalhe).
    for ev in wheel.read() {
        orbit.yaw += ev.y * 0.18;
    }
}

/// Centro do footprint da casa no plano do chão (x, z).
fn house_center_xz() -> Vec2 {
    Vec2::new((HOUSE_X0 + HOUSE_X1) as f32 / 2.0, (HOUSE_Y0 + HOUSE_Y1) as f32 / 2.0)
}

/// "Cone de sombra" da casa visto do boneco: retorna `(base, lo, hi)`, onde `base` é o
/// ângulo (convenção da órbita: `atan2(dx, dz)`) do centro da casa a partir de `char`, e
/// `[lo, hi]` (relativos a `base`) é o arco angular que a casa ocupa. A câmera está ocluída
/// quando seu ângulo (relativo a `base`) cai dentro de `[lo, hi]`. Como o boneco está FORA
/// da casa (convexa), o arco é < π.
fn house_cone(char: Vec2) -> (f32, f32, f32) {
    let center = house_center_xz();
    let base = (center.x - char.x).atan2(center.y - char.y);
    let corners = [
        Vec2::new(HOUSE_X0 as f32 - 0.5, HOUSE_Y0 as f32 - 0.5),
        Vec2::new(HOUSE_X1 as f32 + 0.5, HOUSE_Y0 as f32 - 0.5),
        Vec2::new(HOUSE_X0 as f32 - 0.5, HOUSE_Y1 as f32 + 0.5),
        Vec2::new(HOUSE_X1 as f32 + 0.5, HOUSE_Y1 as f32 + 0.5),
    ];
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for c in corners {
        let d = c - char;
        let a = wrap_angle(d.x.atan2(d.y) - base);
        lo = lo.min(a);
        hi = hi.max(a);
    }
    (base, lo, hi)
}

/// Normaliza um ângulo para `(-π, π]`.
fn wrap_angle(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (a + PI).rem_euclid(TAU) - PI
}

/// Auto-rotação anti-oclusão: se a casa fica ENTRE a câmera e o próprio boneco (e ele está
/// FORA da casa), gira a câmera sozinha — pelo caminho mais curto de se afastar da direção
/// da casa — até a linha de visão ficar livre e o boneco reaparecer.
fn auto_rotate_camera(
    time: Res<Time>,
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut orbit: ResMut<CamOrbit>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, pos)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let char_xz = pos.0;
    // Dentro da casa: a visão interna já cuida — não auto-rotaciona.
    let (cx, cy) = (char_xz.x.round(), char_xz.y.round());
    if cx >= 0.0 && cy >= 0.0 && in_house(cx as u32, cy as u32) {
        orbit.auto_dir = 0.0;
        return;
    }
    // Cone de sombra da casa visto do boneco; `ya` = ângulo da câmera relativo ao centro.
    const M_OFF: f32 = 0.18; // folga angular de saída (~10°)
    let (base, lo, hi) = house_cone(char_xz);
    let ya = wrap_angle(orbit.yaw - base);

    if orbit.auto_dir == 0.0 {
        // Começa só se a câmera estiver DENTRO do cone (boneco tampado pela casa).
        if ya <= lo || ya >= hi {
            return;
        }
        // Escolhe a borda mais PRÓXIMA (caminho mais curto pra sair) — trava o lado.
        orbit.auto_dir = if ya - lo <= hi - ya { -1.0 } else { 1.0 };
    } else if ya <= lo - M_OFF || ya >= hi + M_OFF {
        orbit.auto_dir = 0.0; // já saiu do cone com folga -> para
        return;
    }

    // Alvo = borda escolhida + folga; move a yaw suave até lá (nunca mais que o necessário,
    // então não dá a "volta inteira"). Acompanha a borda enquanto o boneco anda.
    let target_rel = if orbit.auto_dir < 0.0 { lo - M_OFF } else { hi + M_OFF };
    let diff = wrap_angle(base + target_rel - orbit.yaw);
    let step = (2.6 * time.delta_secs()).min(diff.abs());
    orbit.yaw += step * diff.signum();
}

/// A câmera ORBITA o próprio personagem: foco no player, posição a `CAM_DIST` com tilt
/// `pitch` e giro `yaw`. Resolve "seguir o player" + "girar ao redor" de uma vez.
fn camera_orbit(
    orbit: Res<CamOrbit>,
    me: Res<MyPlayer>,
    players: Query<(&Player, &Position)>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<Camera3d>>,
) {
    let Some(my_id) = me.0 else {
        return;
    };
    let Some((_, position)) = players.iter().find(|(p, _)| p.0 == my_id) else {
        return;
    };
    let focus = Vec3::new(position.0.x, 0.8, position.0.y);
    let dir = Quat::from_rotation_y(orbit.yaw)
        * Vec3::new(0.0, orbit.pitch.sin(), orbit.pitch.cos());
    for (mut transform, mut projection) in &mut cameras {
        transform.translation = focus + dir * CAM_DIST;
        transform.look_at(focus, Vec3::Y);
        if let Projection::Orthographic(ortho) = &mut *projection {
            ortho.scaling_mode = ScalingMode::FixedVertical { viewport_height: orbit.zoom };
        }
    }
}
