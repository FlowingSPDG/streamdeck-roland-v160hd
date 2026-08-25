use roland_rs::devices::v160hd::{self, switch};
use roland_rs::{Address, Command, RolandError};

use crate::settings::ActionSettings;

#[allow(dead_code)]
pub const UUID_PREFIX: &str = "com.flowingspdg.roland.v160hd.";

pub const SELECT_PGM: &str = "com.flowingspdg.roland.v160hd.select.pgm";
pub const SELECT_PST: &str = "com.flowingspdg.roland.v160hd.select.pst";
pub const CUT: &str = "com.flowingspdg.roland.v160hd.cut";
pub const AUTO: &str = "com.flowingspdg.roland.v160hd.auto";
pub const PANEL_SWITCH: &str = "com.flowingspdg.roland.v160hd.panel.switch";
pub const INPUT_ASSIGN: &str = "com.flowingspdg.roland.v160hd.input.assign";
pub const OUTPUT_ASSIGN: &str = "com.flowingspdg.roland.v160hd.output.assign";
pub const AUX: &str = "com.flowingspdg.roland.v160hd.aux";
pub const LAYER_ENABLE: &str = "com.flowingspdg.roland.v160hd.layer.enable";
pub const PINP: &str = "com.flowingspdg.roland.v160hd.pinp";
pub const DSK: &str = "com.flowingspdg.roland.v160hd.dsk";
pub const TRANSITION: &str = "com.flowingspdg.roland.v160hd.transition";
pub const MEMORY: &str = "com.flowingspdg.roland.v160hd.memory";
pub const FREEZE: &str = "com.flowingspdg.roland.v160hd.freeze";
pub const MACRO: &str = "com.flowingspdg.roland.v160hd.macro";
pub const CAMERA: &str = "com.flowingspdg.roland.v160hd.camera";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gesture {
    Down,
    Up,
}

#[derive(Debug)]
pub enum DeviceJob {
    Commands(Vec<Command>),
    PressRelease(Address),
    Write(Command),
}

impl DeviceJob {
    fn write(command: Command) -> Self {
        Self::Write(command)
    }

    fn commands2(commands: [Command; 2]) -> Self {
        Self::Commands(commands.to_vec())
    }
}

pub fn build_job(
    action: &str,
    settings: &ActionSettings,
    gesture: Gesture,
) -> Result<Option<DeviceJob>, String> {
    match action {
        SELECT_PGM if gesture == Gesture::Down => Ok(Some(DeviceJob::write(v160hd::select_pgm(
            parse_video_source(&settings.source)?,
        )))),
        SELECT_PST if gesture == Gesture::Down => Ok(Some(DeviceJob::write(v160hd::select_pst(
            parse_video_source(&settings.source)?,
        )))),
        CUT if gesture == Gesture::Down => Ok(Some(DeviceJob::PressRelease(switch::CUT))),
        AUTO if gesture == Gesture::Down => Ok(Some(DeviceJob::PressRelease(switch::AUTO))),
        PANEL_SWITCH => {
            let sw = parse_switch(&settings.switch)?;
            match gesture {
                Gesture::Down => Ok(Some(DeviceJob::write(v160hd::press_switch(sw)))),
                Gesture::Up => Ok(Some(DeviceJob::write(v160hd::release_switch(sw)))),
            }
        }
        INPUT_ASSIGN if gesture == Gesture::Down => {
            Ok(Some(DeviceJob::write(v160hd::assign_input(
                parse_input_channel(&settings.channel)?,
                parse_input_assign(&settings.input_assign)?,
            ))))
        }
        OUTPUT_ASSIGN if gesture == Gesture::Down => {
            Ok(Some(DeviceJob::write(v160hd::assign_output(
                parse_output(&settings.output)?,
                parse_output_assign(&settings.output_assign)?,
            ))))
        }
        AUX if gesture == Gesture::Down => aux_job(settings),
        LAYER_ENABLE if gesture == Gesture::Down => Ok(Some(DeviceJob::write(
            v160hd::set_layer_enable(parse_mix_layer(&settings.mix_layer)?, settings.enable),
        ))),
        PINP if gesture == Gesture::Down => pinp_job(settings),
        DSK if gesture == Gesture::Down => dsk_job(settings),
        TRANSITION if gesture == Gesture::Down => transition_job(settings),
        MEMORY if gesture == Gesture::Down => memory_job(settings),
        FREEZE if gesture == Gesture::Down => freeze_job(settings),
        MACRO if gesture == Gesture::Down => {
            let n = parse_u8(&settings.macro_n, "macro")?;
            Ok(Some(DeviceJob::write(v160hd::run_macro(n).map_err(err)?)))
        }
        CAMERA => camera_job(settings, gesture),
        _ if gesture == Gesture::Up => Ok(None),
        _ => Err(format!("unknown action {action}")),
    }
}

fn aux_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    let aux = parse_aux_bus(&settings.aux_bus)?;
    match settings.aux_op.as_str() {
        "source" => Ok(Some(DeviceJob::write(v160hd::assign_aux(
            aux,
            parse_video_source(&settings.source)?,
        )))),
        "mute" => Ok(Some(DeviceJob::write(v160hd::mute_aux(
            aux,
            settings.muted,
        )))),
        "linked" => Ok(Some(DeviceJob::write(v160hd::set_aux_linked(
            aux,
            settings.linked,
        )))),
        "link_mode" => Ok(Some(DeviceJob::write(v160hd::set_aux_link_mode(
            parse_aux_link_mode(&settings.link_mode)?,
        )))),
        other => Err(format!("unknown aux operation {other}")),
    }
}

fn pinp_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    let key = parse_pinp_key(&settings.pinp_key)?;
    match settings.pinp_op.as_str() {
        "fade" => Ok(Some(DeviceJob::write(v160hd::set_pinp_fade(
            key,
            settings.enable,
        )))),
        "bus" => Ok(Some(DeviceJob::write(v160hd::set_pinp_bus(
            key,
            parse_mix_bus(&settings.bus)?,
            settings.enable,
        )))),
        "source" => Ok(Some(DeviceJob::write(v160hd::set_pinp_source(
            key,
            parse_video_source(&settings.source)?,
        )))),
        "type" => Ok(Some(DeviceJob::write(v160hd::set_pinp_type(
            key,
            parse_pinp_type(&settings.pinp_type)?,
        )))),
        "position_h" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_position_h(
            key,
            parse_i32(&settings.value, "value")?,
        )))),
        "position_v" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_position_v(
            key,
            parse_i32(&settings.value, "value")?,
        )))),
        "size" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_size(
            key,
            parse_i32(&settings.value, "value")?,
        )))),
        "crop_h" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_crop_h(
            key,
            parse_i32(&settings.value, "value")?,
        )))),
        "crop_v" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_crop_v(
            key,
            parse_i32(&settings.value, "value")?,
        )))),
        "shape" => Ok(Some(DeviceJob::write(v160hd::set_pinp_shape(
            key,
            parse_shape(&settings.shape)?,
        )))),
        "border_color" => Ok(Some(DeviceJob::write(v160hd::set_pinp_border_color(
            key,
            parse_border_color(&settings.border_color)?,
        )))),
        "border_width" => Ok(Some(DeviceJob::write(v160hd::set_pinp_border_width(
            key,
            parse_u8(&settings.value, "value")?,
        )))),
        "view_h" => Ok(Some(DeviceJob::commands2(
            v160hd::set_pinp_view_position_h(key, parse_i32(&settings.value, "value")?),
        ))),
        "view_v" => Ok(Some(DeviceJob::commands2(
            v160hd::set_pinp_view_position_v(key, parse_i32(&settings.value, "value")?),
        ))),
        "view_zoom" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_view_zoom(
            key,
            parse_u16(&settings.value, "value")?,
        )))),
        "key_level" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_key_level(
            key,
            parse_u8(&settings.value, "value")?,
        )))),
        "key_gain" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_key_gain(
            key,
            parse_u8(&settings.value, "value")?,
        )))),
        "mix_level" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_mix_level(
            key,
            parse_u8(&settings.value, "value")?,
        )))),
        "chroma" => Ok(Some(DeviceJob::write(v160hd::set_pinp_chroma_color(
            key,
            parse_chroma(&settings.chroma_color)?,
        )))),
        "hue_width" => Ok(Some(DeviceJob::write(v160hd::set_pinp_hue_width(
            key,
            parse_i8(&settings.value, "value")?,
        )))),
        "hue_fine" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_hue_fine(
            key,
            parse_u16(&settings.value, "value")?,
        )))),
        "sat_width" => Ok(Some(DeviceJob::commands2(
            v160hd::set_pinp_saturation_width(key, parse_i16(&settings.value, "value")?),
        ))),
        "sat_fine" => Ok(Some(DeviceJob::commands2(
            v160hd::set_pinp_saturation_fine(key, parse_u8(&settings.value, "value")?),
        ))),
        "border_r" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_border_red(
            key,
            parse_u8(&settings.value, "value")?,
        )))),
        "border_g" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_border_green(
            key,
            parse_u8(&settings.value, "value")?,
        )))),
        "border_b" => Ok(Some(DeviceJob::commands2(v160hd::set_pinp_border_blue(
            key,
            parse_u8(&settings.value, "value")?,
        )))),
        other => Err(format!("unknown PinP operation {other}")),
    }
}

fn dsk_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    let ch = parse_dsk(&settings.dsk_ch)?;
    match settings.dsk_op.as_str() {
        "bus" => Ok(Some(DeviceJob::write(v160hd::set_dsk_bus(
            ch,
            parse_mix_bus(&settings.bus)?,
            settings.enable,
        )))),
        "key_source" => Ok(Some(DeviceJob::write(v160hd::set_dsk_key_source(
            ch,
            parse_video_source(&settings.source)?,
        )))),
        "fill_source" => Ok(Some(DeviceJob::write(v160hd::set_dsk_fill_source(
            ch,
            parse_video_source(&settings.source)?,
        )))),
        "type" => Ok(Some(DeviceJob::write(v160hd::set_dsk_type(
            ch,
            parse_dsk_type(&settings.dsk_type)?,
        )))),
        other => Err(format!("unknown DSK operation {other}")),
    }
}

fn transition_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    match settings.trans_op.as_str() {
        "time" => Ok(Some(DeviceJob::write(v160hd::set_transition_time(
            parse_trans_time(&settings.trans_time)?,
            parse_u8(&settings.tenths, "tenths")?,
        )))),
        "type" => Ok(Some(DeviceJob::write(v160hd::set_transition_type(
            parse_trans_type(&settings.trans_type)?,
        )))),
        "mix_type" => Ok(Some(DeviceJob::write(v160hd::set_mix_type(
            parse_mix_type(&settings.mix_type)?,
        )))),
        "wipe_type" => Ok(Some(DeviceJob::write(v160hd::set_wipe_type(
            parse_wipe_type(&settings.wipe_type)?,
        )))),
        "wipe_direction" => Ok(Some(DeviceJob::write(v160hd::set_wipe_direction(
            parse_wipe_dir(&settings.wipe_direction)?,
        )))),
        other => Err(format!("unknown transition operation {other}")),
    }
}

fn memory_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    let slot = v160hd::MemorySlot::new(parse_u8(&settings.slot, "slot")?).map_err(err)?;
    match settings.mem_op.as_str() {
        "load" => Ok(Some(DeviceJob::write(v160hd::load_memory(slot)))),
        "save" => Ok(Some(DeviceJob::write(v160hd::save_memory(slot)))),
        "init" => Ok(Some(DeviceJob::write(v160hd::init_memory(slot)))),
        other => Err(format!("unknown memory operation {other}")),
    }
}

fn freeze_job(settings: &ActionSettings) -> Result<Option<DeviceJob>, String> {
    match settings.freeze_op.as_str() {
        "on" => Ok(Some(DeviceJob::write(v160hd::set_freeze(settings.enable)))),
        "type" => Ok(Some(DeviceJob::write(v160hd::set_freeze_type(
            parse_freeze_type(&settings.freeze_type)?,
        )))),
        "select" => Ok(Some(DeviceJob::write(v160hd::set_freeze_select(
            parse_freeze_input(&settings.freeze_input)?,
            settings.enable,
        )))),
        other => Err(format!("unknown freeze operation {other}")),
    }
}

fn camera_job(settings: &ActionSettings, gesture: Gesture) -> Result<Option<DeviceJob>, String> {
    let cam = v160hd::CameraId::new(parse_u8(&settings.camera_id, "camera")?).map_err(err)?;
    match settings.cam_op.as_str() {
        "preset" if gesture == Gesture::Down => Ok(Some(DeviceJob::write(v160hd::camera_preset(
            cam,
            parse_camera_preset(&settings.preset)?,
        )))),
        "pan" => Ok(Some(DeviceJob::write(v160hd::camera_pan(
            cam,
            match gesture {
                Gesture::Down => parse_pan(&settings.pan)?,
                Gesture::Up => v160hd::PanDirection::Stop,
            },
        )))),
        "tilt" => Ok(Some(DeviceJob::write(v160hd::camera_tilt(
            cam,
            match gesture {
                Gesture::Down => parse_tilt(&settings.tilt)?,
                Gesture::Up => v160hd::TiltDirection::Stop,
            },
        )))),
        "zoom" => Ok(Some(DeviceJob::write(v160hd::camera_zoom(
            cam,
            match gesture {
                Gesture::Down => parse_zoom(&settings.zoom)?,
                Gesture::Up => v160hd::ZoomCommand::Stop,
            },
        )))),
        "focus" => Ok(Some(DeviceJob::write(v160hd::camera_focus(
            cam,
            match gesture {
                Gesture::Down => parse_focus(&settings.focus)?,
                Gesture::Up => v160hd::FocusCommand::Stop,
            },
        )))),
        "pt_speed" if gesture == Gesture::Down => Ok(Some(DeviceJob::write(
            v160hd::camera_pt_speed(cam, parse_u8(&settings.pt_speed, "speed")?),
        ))),
        "auto_focus" if gesture == Gesture::Down => Ok(Some(DeviceJob::write(
            v160hd::camera_auto_focus(cam, settings.auto_focus),
        ))),
        "exposure" if gesture == Gesture::Down => Ok(Some(DeviceJob::write(
            v160hd::camera_exposure_auto(cam, settings.exposure_auto),
        ))),
        "tally" if gesture == Gesture::Down => Ok(Some(DeviceJob::write(
            v160hd::camera_tally_channel(cam, parse_video_source(&settings.source)?),
        ))),
        _ if gesture == Gesture::Up => Ok(None),
        other => Err(format!("unknown camera operation {other}")),
    }
}

fn err(e: RolandError) -> String {
    e.to_string()
}

fn parse_u8(s: &str, name: &str) -> Result<u8, String> {
    s.trim().parse().map_err(|_| format!("invalid {name}: {s}"))
}

fn parse_u16(s: &str, name: &str) -> Result<u16, String> {
    s.trim().parse().map_err(|_| format!("invalid {name}: {s}"))
}

fn parse_i8(s: &str, name: &str) -> Result<i8, String> {
    s.trim().parse().map_err(|_| format!("invalid {name}: {s}"))
}

fn parse_i16(s: &str, name: &str) -> Result<i16, String> {
    s.trim().parse().map_err(|_| format!("invalid {name}: {s}"))
}

fn parse_i32(s: &str, name: &str) -> Result<i32, String> {
    s.trim().parse().map_err(|_| format!("invalid {name}: {s}"))
}

pub fn parse_video_source(s: &str) -> Result<v160hd::VideoSource, String> {
    let (kind, n) = s
        .split_once(':')
        .ok_or_else(|| format!("invalid video source {s}"))?;
    let n: u8 = n.parse().map_err(|_| format!("invalid video source {s}"))?;
    match kind {
        "hdmi" => v160hd::VideoSource::hdmi(n),
        "sdi" => v160hd::VideoSource::sdi(n),
        "still" => v160hd::VideoSource::still(n),
        "input" => v160hd::VideoSource::input(n),
        _ => return Err(format!("invalid video source {s}")),
    }
    .map_err(err)
}

fn parse_input_channel(s: &str) -> Result<v160hd::InputChannel, String> {
    v160hd::InputChannel::new(parse_u8(s, "channel")?).map_err(err)
}

fn parse_input_assign(s: &str) -> Result<v160hd::InputAssign, String> {
    use v160hd::InputAssign::*;
    Ok(match s {
        "hdmi1" => Hdmi1,
        "hdmi2" => Hdmi2,
        "hdmi3" => Hdmi3,
        "hdmi4" => Hdmi4,
        "hdmi5" => Hdmi5,
        "hdmi6" => Hdmi6,
        "hdmi7" => Hdmi7,
        "hdmi8" => Hdmi8,
        "sdi1" => Sdi1,
        "sdi2" => Sdi2,
        "sdi3" => Sdi3,
        "sdi4" => Sdi4,
        "sdi5" => Sdi5,
        "sdi6" => Sdi6,
        "sdi7" => Sdi7,
        "sdi8" => Sdi8,
        "still1" => Still1,
        "still2" => Still2,
        "still3" => Still3,
        "still4" => Still4,
        "still5" => Still5,
        "still6" => Still6,
        "still7" => Still7,
        "still8" => Still8,
        "still9" => Still9,
        "still10" => Still10,
        "still11" => Still11,
        "still12" => Still12,
        "still13" => Still13,
        "still14" => Still14,
        "still15" => Still15,
        "still16" => Still16,
        other => return Err(format!("invalid input assign {other}")),
    })
}

fn parse_output(s: &str) -> Result<v160hd::Output, String> {
    Ok(match s {
        "hdmi1" => v160hd::Output::Hdmi1,
        "hdmi2" => v160hd::Output::Hdmi2,
        "hdmi3" => v160hd::Output::Hdmi3,
        "sdi1" => v160hd::Output::Sdi1,
        "sdi2" => v160hd::Output::Sdi2,
        "sdi3" => v160hd::Output::Sdi3,
        "usb" => v160hd::Output::Usb,
        other => return Err(format!("invalid output {other}")),
    })
}

fn parse_output_assign(s: &str) -> Result<v160hd::OutputAssign, String> {
    use v160hd::OutputAssign::*;
    Ok(match s {
        "program" => Program,
        "sub_program" => SubProgram,
        "preview" => Preview,
        "aux1" => Aux1,
        "aux2" => Aux2,
        "aux3" => Aux3,
        "dsk1" => Dsk1Source,
        "dsk2" => Dsk2Source,
        "multiview" => MultiView,
        "input16" => Input16View,
        "still16" => Still16View,
        other => return Err(format!("invalid output assign {other}")),
    })
}

fn parse_aux_bus(s: &str) -> Result<v160hd::AuxBus, String> {
    Ok(match s {
        "1" | "aux1" => v160hd::AuxBus::Aux1,
        "2" | "aux2" => v160hd::AuxBus::Aux2,
        "3" | "aux3" => v160hd::AuxBus::Aux3,
        other => return Err(format!("invalid AUX {other}")),
    })
}

fn parse_aux_link_mode(s: &str) -> Result<v160hd::AuxLinkMode, String> {
    Ok(match s {
        "off" => v160hd::AuxLinkMode::Off,
        "auto" => v160hd::AuxLinkMode::AutoLink,
        "manual" => v160hd::AuxLinkMode::ManualLink,
        other => return Err(format!("invalid AUX link mode {other}")),
    })
}

fn parse_mix_layer(s: &str) -> Result<v160hd::MixLayer, String> {
    use v160hd::MixLayer::*;
    Ok(match s {
        "pgm_pinp1" => PgmPinP1,
        "pgm_pinp2" => PgmPinP2,
        "pgm_pinp3" => PgmPinP3,
        "pgm_pinp4" => PgmPinP4,
        "pgm_dsk1" => PgmDsk1,
        "pgm_dsk2" => PgmDsk2,
        "sub_pinp1" => SubPgmPinP1,
        "sub_pinp2" => SubPgmPinP2,
        "sub_pinp3" => SubPgmPinP3,
        "sub_pinp4" => SubPgmPinP4,
        "sub_dsk1" => SubPgmDsk1,
        "sub_dsk2" => SubPgmDsk2,
        other => return Err(format!("invalid mix layer {other}")),
    })
}

fn parse_pinp_key(s: &str) -> Result<v160hd::PinPKey, String> {
    v160hd::PinPKey::from_index(parse_u8(s, "pinp")?).map_err(err)
}

fn parse_mix_bus(s: &str) -> Result<v160hd::MixBus, String> {
    Ok(match s {
        "pgm" | "program" => v160hd::MixBus::Program,
        "pvw" | "preview" => v160hd::MixBus::Preview,
        other => return Err(format!("invalid bus {other}")),
    })
}

fn parse_pinp_type(s: &str) -> Result<v160hd::PinPType, String> {
    Ok(match s {
        "pinp" => v160hd::PinPType::PinP,
        "luma_white" => v160hd::PinPType::LuminanceWhite,
        "luma_black" => v160hd::PinPType::LuminanceBlack,
        "chroma" => v160hd::PinPType::Chroma,
        other => return Err(format!("invalid PinP type {other}")),
    })
}

fn parse_shape(s: &str) -> Result<v160hd::PinPShape, String> {
    Ok(match s {
        "rect" => v160hd::PinPShape::Rectangle,
        "circle" => v160hd::PinPShape::Circle,
        "diamond" => v160hd::PinPShape::Diamond,
        other => return Err(format!("invalid shape {other}")),
    })
}

fn parse_border_color(s: &str) -> Result<v160hd::BorderColor, String> {
    use v160hd::BorderColor::*;
    Ok(match s {
        "white" => White,
        "yellow" => Yellow,
        "cyan" => Cyan,
        "green" => Green,
        "magenta" => Magenta,
        "red" => Red,
        "blue" => Blue,
        "black" => Black,
        "custom" => Custom,
        "soft" => SoftEdge,
        other => return Err(format!("invalid border color {other}")),
    })
}

fn parse_chroma(s: &str) -> Result<v160hd::ChromaColor, String> {
    Ok(match s {
        "green" => v160hd::ChromaColor::Green,
        "blue" => v160hd::ChromaColor::Blue,
        other => return Err(format!("invalid chroma {other}")),
    })
}

fn parse_dsk(s: &str) -> Result<v160hd::DskChannel, String> {
    v160hd::DskChannel::from_index(parse_u8(s, "dsk")?).map_err(err)
}

fn parse_dsk_type(s: &str) -> Result<v160hd::DskType, String> {
    Ok(match s {
        "luma_white" => v160hd::DskType::LuminanceWhite,
        "luma_black" => v160hd::DskType::LuminanceBlack,
        "chroma" => v160hd::DskType::Chroma,
        other => return Err(format!("invalid DSK type {other}")),
    })
}

fn parse_trans_time(s: &str) -> Result<v160hd::TransitionTime, String> {
    use v160hd::TransitionTime::*;
    Ok(match s {
        "mix_wipe" => MixWipe,
        "pinp1" => PinP1,
        "pinp2" => PinP2,
        "pinp3" => PinP3,
        "pinp4" => PinP4,
        "dsk1" => Dsk1,
        "dsk2" => Dsk2,
        "output_fade" => OutputFade,
        other => return Err(format!("invalid transition time target {other}")),
    })
}

fn parse_trans_type(s: &str) -> Result<v160hd::TransitionType, String> {
    Ok(match s {
        "mix" => v160hd::TransitionType::Mix,
        "wipe" => v160hd::TransitionType::Wipe,
        other => return Err(format!("invalid transition type {other}")),
    })
}

fn parse_mix_type(s: &str) -> Result<v160hd::MixType, String> {
    Ok(match s {
        "mix" => v160hd::MixType::Mix,
        "fam" => v160hd::MixType::Fam,
        "nam" => v160hd::MixType::Nam,
        other => return Err(format!("invalid mix type {other}")),
    })
}

fn parse_wipe_type(s: &str) -> Result<v160hd::WipeType, String> {
    use v160hd::WipeType::*;
    Ok(match s {
        "h" => Horizontal,
        "v" => Vertical,
        "ul" => UpperLeft,
        "ur" => UpperRight,
        "ll" => LowerLeft,
        "lr" => LowerRight,
        "hc" => HCenter,
        "vc" => VCenter,
        other => return Err(format!("invalid wipe type {other}")),
    })
}

fn parse_wipe_dir(s: &str) -> Result<v160hd::WipeDirection, String> {
    Ok(match s {
        "normal" => v160hd::WipeDirection::Normal,
        "reverse" => v160hd::WipeDirection::Reverse,
        "round" => v160hd::WipeDirection::RoundTrip,
        other => return Err(format!("invalid wipe direction {other}")),
    })
}

fn parse_freeze_type(s: &str) -> Result<v160hd::FreezeType, String> {
    Ok(match s {
        "all" => v160hd::FreezeType::All,
        "select" => v160hd::FreezeType::Select,
        other => return Err(format!("invalid freeze type {other}")),
    })
}

fn parse_freeze_input(s: &str) -> Result<v160hd::FreezeInput, String> {
    let (kind, n) = s
        .split_once(':')
        .ok_or_else(|| format!("invalid freeze input {s}"))?;
    let n: u8 = n.parse().map_err(|_| format!("invalid freeze input {s}"))?;
    match kind {
        "hdmi" => v160hd::FreezeInput::hdmi(n),
        "sdi" => v160hd::FreezeInput::sdi(n),
        _ => return Err(format!("invalid freeze input {s}")),
    }
    .map_err(err)
}

fn parse_camera_preset(s: &str) -> Result<v160hd::CameraPreset, String> {
    if s == "none" {
        return Ok(v160hd::CameraPreset::NONE);
    }
    v160hd::CameraPreset::new(parse_u8(s, "preset")?).map_err(err)
}

fn parse_pan(s: &str) -> Result<v160hd::PanDirection, String> {
    Ok(match s {
        "left" => v160hd::PanDirection::Left,
        "right" => v160hd::PanDirection::Right,
        "stop" => v160hd::PanDirection::Stop,
        other => return Err(format!("invalid pan {other}")),
    })
}

fn parse_tilt(s: &str) -> Result<v160hd::TiltDirection, String> {
    Ok(match s {
        "up" => v160hd::TiltDirection::Up,
        "down" => v160hd::TiltDirection::Down,
        "stop" => v160hd::TiltDirection::Stop,
        other => return Err(format!("invalid tilt {other}")),
    })
}

fn parse_zoom(s: &str) -> Result<v160hd::ZoomCommand, String> {
    Ok(match s {
        "in_slow" => v160hd::ZoomCommand::InSlow,
        "in_fast" => v160hd::ZoomCommand::InFast,
        "out_slow" => v160hd::ZoomCommand::OutSlow,
        "out_fast" => v160hd::ZoomCommand::OutFast,
        "stop" => v160hd::ZoomCommand::Stop,
        other => return Err(format!("invalid zoom {other}")),
    })
}

fn parse_focus(s: &str) -> Result<v160hd::FocusCommand, String> {
    Ok(match s {
        "far" => v160hd::FocusCommand::Far,
        "near" => v160hd::FocusCommand::Near,
        "stop" => v160hd::FocusCommand::Stop,
        other => return Err(format!("invalid focus {other}")),
    })
}

fn parse_switch(s: &str) -> Result<Address, String> {
    Ok(match s {
        "pgm_a_1" => switch::PGM_A_1,
        "pgm_a_2" => switch::PGM_A_2,
        "pgm_a_3" => switch::PGM_A_3,
        "pgm_a_4" => switch::PGM_A_4,
        "pgm_a_5" => switch::PGM_A_5,
        "pgm_a_6" => switch::PGM_A_6,
        "pgm_a_7" => switch::PGM_A_7,
        "pgm_a_8" => switch::PGM_A_8,
        "pgm_a_9" => switch::PGM_A_9,
        "pgm_a_10" => switch::PGM_A_10,
        "pst_b_1" => switch::PST_B_1,
        "pst_b_2" => switch::PST_B_2,
        "pst_b_3" => switch::PST_B_3,
        "pst_b_4" => switch::PST_B_4,
        "pst_b_5" => switch::PST_B_5,
        "pst_b_6" => switch::PST_B_6,
        "pst_b_7" => switch::PST_B_7,
        "pst_b_8" => switch::PST_B_8,
        "pst_b_9" => switch::PST_B_9,
        "pst_b_10" => switch::PST_B_10,
        "aux_1" => switch::AUX_1,
        "aux_2" => switch::AUX_2,
        "aux_3" => switch::AUX_3,
        "aux_4" => switch::AUX_4,
        "aux_5" => switch::AUX_5,
        "aux_6" => switch::AUX_6,
        "aux_7" => switch::AUX_7,
        "aux_8" => switch::AUX_8,
        "aux_9" => switch::AUX_9,
        "aux_10" => switch::AUX_10,
        "cut" => switch::CUT,
        "auto" => switch::AUTO,
        "transition" => switch::TRANSITION,
        "mode" => switch::MODE,
        "input_assign" => switch::INPUT_ASSIGN,
        "pgm_center" => switch::PGM_CENTER_ENCODER,
        "pst_center" => switch::PST_CENTER_ENCODER,
        "split_a" => switch::SPLIT_A,
        "split_b" => switch::SPLIT_B,
        "auto_mixing" => switch::AUTO_MIXING,
        "capture" => switch::CAPTURE,
        "user_1" => switch::USER_1,
        "user_2" => switch::USER_2,
        "user_3" => switch::USER_3,
        "user_4" => switch::USER_4,
        "pinp1_pos_h" => switch::PINP1_POS_H,
        "pinp1_pos_v" => switch::PINP1_POS_V,
        "pinp1_source" => switch::PINP1_SOURCE,
        "pinp1_pvw" => switch::PINP1_PVW,
        "pinp1_pgm" => switch::PINP1_PGM,
        "pinp2_pos_h" => switch::PINP2_POS_H,
        "pinp2_pos_v" => switch::PINP2_POS_V,
        "pinp2_source" => switch::PINP2_SOURCE,
        "pinp2_pvw" => switch::PINP2_PVW,
        "pinp2_pgm" => switch::PINP2_PGM,
        "pinp3_pos_h" => switch::PINP3_POS_H,
        "pinp3_pos_v" => switch::PINP3_POS_V,
        "pinp3_source" => switch::PINP3_SOURCE,
        "pinp3_pvw" => switch::PINP3_PVW,
        "pinp3_pgm" => switch::PINP3_PGM,
        "pinp4_pos_h" => switch::PINP4_POS_H,
        "pinp4_pos_v" => switch::PINP4_POS_V,
        "pinp4_source" => switch::PINP4_SOURCE,
        "pinp4_pvw" => switch::PINP4_PVW,
        "pinp4_pgm" => switch::PINP4_PGM,
        "dsk1_source" => switch::DSK1_SOURCE,
        "dsk1_pvw" => switch::DSK1_PVW,
        "dsk1_pgm" => switch::DSK1_PGM,
        "dsk2_source" => switch::DSK2_SOURCE,
        "dsk2_pvw" => switch::DSK2_PVW,
        "dsk2_pgm" => switch::DSK2_PGM,
        "monitor_1" => switch::MONITOR_1,
        "monitor_2" => switch::MONITOR_2,
        "monitor_3" => switch::MONITOR_3,
        "monitor_4" => switch::MONITOR_4,
        "menu" => switch::MENU,
        "exit" => switch::EXIT,
        "enter" => switch::ENTER,
        "output_fade" => switch::OUTPUT_FADE,
        "sequencer_on" => switch::SEQUENCER_ON,
        "sequencer_auto" => switch::SEQUENCER_AUTO,
        "sequencer_prev" => switch::SEQUENCER_PREV,
        "sequencer_next" => switch::SEQUENCER_NEXT,
        other => return Err(format!("invalid switch {other}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgm_hdmi1_encodes_official_style() {
        let settings = ActionSettings {
            source: "hdmi:1".into(),
            ..ActionSettings::default()
        };
        let job = build_job(SELECT_PGM, &settings, Gesture::Down)
            .unwrap()
            .unwrap();
        match job {
            DeviceJob::Write(cmd) => assert_eq!(cmd.encode(), "DTH:002100,00;"),
            _ => panic!("expected write"),
        }
    }

    #[test]
    fn cut_is_press_and_release() {
        let job = build_job(CUT, &ActionSettings::default(), Gesture::Down)
            .unwrap()
            .unwrap();
        match job {
            DeviceJob::PressRelease(addr) => assert_eq!(addr, switch::CUT),
            _ => panic!("expected press/release"),
        }
        assert!(build_job(CUT, &ActionSettings::default(), Gesture::Up)
            .unwrap()
            .is_none());
    }

    #[test]
    fn camera_pan_stops_on_keyup() {
        let settings = ActionSettings {
            camera_id: "1".into(),
            cam_op: "pan".into(),
            pan: "left".into(),
            ..ActionSettings::default()
        };
        let up = build_job(CAMERA, &settings, Gesture::Up).unwrap().unwrap();
        match up {
            DeviceJob::Write(cmd) => assert_eq!(cmd.encode(), "DTH:024122,00;"),
            _ => panic!("expected write"),
        }
    }

    #[test]
    fn pinp_position_is_two_commands() {
        let settings = ActionSettings {
            pinp_key: "1".into(),
            pinp_op: "position_h".into(),
            value: "-1000".into(),
            ..ActionSettings::default()
        };
        let job = build_job(PINP, &settings, Gesture::Down).unwrap().unwrap();
        match job {
            DeviceJob::Commands(cmds) => assert_eq!(cmds.len(), 2),
            _ => panic!("expected two writes"),
        }
    }
}
