#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    WrongCommand { expected: u8, actual: u8 },
    WrongPayloadSize { expected: usize, actual: usize },
    InvalidPayloadSize(u8),
    InvalidValue { field: &'static str, value: u8 },
    PayloadTooLarge(usize),
    DivisionBlockTooLarge { max: usize, actual: usize },
}

pub type Result<T> = std::result::Result<T, CommandError>;

pub trait Request {
    fn command_id(&self) -> u8;
    fn to_bytes(&self) -> Vec<u8>;
}

pub trait Notify: Sized {
    const CMD: u8;
    const SIZE: usize;

    fn from_ops(ops: &[u8]) -> Result<Self>;

    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let actual_cmd = *bytes.first().ok_or(CommandError::WrongPayloadSize {
            expected: Self::SIZE + 2,
            actual: 0,
        })?;

        if actual_cmd != Self::CMD {
            return Err(CommandError::WrongCommand {
                expected: Self::CMD,
                actual: actual_cmd,
            });
        }

        let actual_size = *bytes.get(1).ok_or(CommandError::WrongPayloadSize {
            expected: Self::SIZE + 2,
            actual: 1,
        })? as usize;

        if actual_size != Self::SIZE {
            return Err(CommandError::WrongPayloadSize {
                expected: Self::SIZE,
                actual: actual_size,
            });
        }

        let ops = bytes.get(2..).ok_or(CommandError::WrongPayloadSize {
            expected: Self::SIZE + 2,
            actual: bytes.len(),
        })?;

        if ops.len() != Self::SIZE {
            return Err(CommandError::WrongPayloadSize {
                expected: Self::SIZE,
                actual: ops.len(),
            });
        }

        Self::from_ops(ops)
    }
}

pub trait FixedSizeRequest {
    const CMD: u8;

    fn ops(&self) -> Vec<u8>;
}

impl<T> Request for T
where
    T: FixedSizeRequest,
{
    fn command_id(&self) -> u8 {
        T::CMD
    }

    fn to_bytes(&self) -> Vec<u8> {
        command_with_sized_ops(T::CMD, self.ops())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CXN0102Notify {
    BootCompleted(BootCompletedNotify),
    StartInput(CommandResultNotify<0x01>),
    StopInput(CommandResultNotify<0x02>),
    StopInputSpecially(CommandResultNotify<0x0C>),
    MuteUnmuteChangeOutput(CommandResultNotify<0x03>),
    SaveUserParam(CommandResultNotify<0x07>),
    InitializeUserParam(CommandResultNotify<0x08>),
    ShutdownReboot(CommandResultNotify<0x0B>),
    GetVideoOutputPosition(GetVideoOutputPositionNotify),
    SetVideoOutputPosition(ErrorResultNotify<0x26>),
    GetOpticalAlignment(GetOpticalAlignmentNotify),
    SetOpticalAlignment(ErrorResultNotify<0x28>),
    GetBiphase(GetBiphaseNotify),
    SetBiphase(ErrorResultNotify<0x2A>),
    EasyOpticalAdjustmentControl(AdjustmentControlNotify<0x32>),
    EasyOpticalAdjustmentPlus(AdjustmentStepNotify<0x33>),
    EasyOpticalAdjustmentMinus(AdjustmentStepNotify<0x34>),
    EasyOpticalAdjustmentExit(CommandResultNotify<0x35>),
    EasyBiphaseAdjustmentControl(AdjustmentControlNotify<0x36>),
    EasyBiphaseAdjustmentPlus(AdjustmentStepNotify<0x37>),
    EasyBiphaseAdjustmentMinus(AdjustmentStepNotify<0x38>),
    EasyBiphaseAdjustmentExit(CommandResultNotify<0x39>),
    GetAllPictureQuality(GetAllPictureQualityNotify),
    SetAllPictureQuality(ErrorResultNotify<0x41>),
    GetBrightness(GetBrightnessNotify),
    SetBrightness(ErrorResultNotify<0x43>),
    GetContrast(GetContrastNotify),
    SetContrast(ErrorResultNotify<0x45>),
    GetHue(GetHueNotify),
    SetHue(ErrorResultNotify<0x47>),
    GetSaturation(GetSaturationNotify),
    SetSaturation(ErrorResultNotify<0x49>),
    GetSharpness(GetSharpnessNotify),
    SetSharpness(ErrorResultNotify<0x4F>),
    UpdateFwImage(UpdateResultNotify<0x82>),
    UpdatePictureData(UpdateResultNotify<0x84>),
    DivisionTransmissionUpdateFwImage(UpdateResultNotify<0x92>),
    DivisionTransmissionUpdatePictureData(UpdateResultNotify<0x94>),
    GetTemperature(GetTemperatureNotify),
    GetTime(GetTimeNotify),
    GetVersion(GetVersionNotify),
    OutputTestPicture(ErrorResultNotify<0xA3>),
    GetLotNumber(GetLotNumberNotify),
    GetSerialNumber(GetSerialNumberNotify),
    Emergency(EmergencyNotify),
    TemperatureEmergencyAndRecovery(TemperatureEmergencyAndRecoveryNotify),
    CommandEmergency(CommandEmergencyNotify),
}

impl CXN0102Notify {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let cmd = *bytes.first().ok_or(CommandError::WrongPayloadSize {
            expected: 2,
            actual: 0,
        })?;

        Ok(match cmd {
            0x00 => Self::BootCompleted(BootCompletedNotify::from_bytes(bytes)?),
            0x01 => Self::StartInput(CommandResultNotify::<0x01>::from_bytes(bytes)?),
            0x02 => Self::StopInput(CommandResultNotify::<0x02>::from_bytes(bytes)?),
            0x03 => Self::MuteUnmuteChangeOutput(CommandResultNotify::<0x03>::from_bytes(bytes)?),
            0x07 => Self::SaveUserParam(CommandResultNotify::<0x07>::from_bytes(bytes)?),
            0x08 => Self::InitializeUserParam(CommandResultNotify::<0x08>::from_bytes(bytes)?),
            0x0B => Self::ShutdownReboot(CommandResultNotify::<0x0B>::from_bytes(bytes)?),
            0x0C => Self::StopInputSpecially(CommandResultNotify::<0x0C>::from_bytes(bytes)?),
            0x10 => Self::Emergency(EmergencyNotify::from_bytes(bytes)?),
            0x11 => Self::TemperatureEmergencyAndRecovery(
                TemperatureEmergencyAndRecoveryNotify::from_bytes(bytes)?,
            ),
            0x12 => Self::CommandEmergency(CommandEmergencyNotify::from_bytes(bytes)?),
            0x25 => Self::GetVideoOutputPosition(GetVideoOutputPositionNotify::from_bytes(bytes)?),
            0x26 => Self::SetVideoOutputPosition(ErrorResultNotify::<0x26>::from_bytes(bytes)?),
            0x27 => Self::GetOpticalAlignment(GetOpticalAlignmentNotify::from_bytes(bytes)?),
            0x28 => Self::SetOpticalAlignment(ErrorResultNotify::<0x28>::from_bytes(bytes)?),
            0x29 => Self::GetBiphase(GetBiphaseNotify::from_bytes(bytes)?),
            0x2A => Self::SetBiphase(ErrorResultNotify::<0x2A>::from_bytes(bytes)?),
            0x32 => Self::EasyOpticalAdjustmentControl(
                AdjustmentControlNotify::<0x32>::from_bytes(bytes)?,
            ),
            0x33 => {
                Self::EasyOpticalAdjustmentPlus(AdjustmentStepNotify::<0x33>::from_bytes(bytes)?)
            }
            0x34 => {
                Self::EasyOpticalAdjustmentMinus(AdjustmentStepNotify::<0x34>::from_bytes(bytes)?)
            }
            0x35 => {
                Self::EasyOpticalAdjustmentExit(CommandResultNotify::<0x35>::from_bytes(bytes)?)
            }
            0x36 => Self::EasyBiphaseAdjustmentControl(
                AdjustmentControlNotify::<0x36>::from_bytes(bytes)?,
            ),
            0x37 => {
                Self::EasyBiphaseAdjustmentPlus(AdjustmentStepNotify::<0x37>::from_bytes(bytes)?)
            }
            0x38 => {
                Self::EasyBiphaseAdjustmentMinus(AdjustmentStepNotify::<0x38>::from_bytes(bytes)?)
            }
            0x39 => {
                Self::EasyBiphaseAdjustmentExit(CommandResultNotify::<0x39>::from_bytes(bytes)?)
            }
            0x40 => Self::GetAllPictureQuality(GetAllPictureQualityNotify::from_bytes(bytes)?),
            0x41 => Self::SetAllPictureQuality(ErrorResultNotify::<0x41>::from_bytes(bytes)?),
            0x42 => Self::GetBrightness(GetBrightnessNotify::from_bytes(bytes)?),
            0x43 => Self::SetBrightness(ErrorResultNotify::<0x43>::from_bytes(bytes)?),
            0x44 => Self::GetContrast(GetContrastNotify::from_bytes(bytes)?),
            0x45 => Self::SetContrast(ErrorResultNotify::<0x45>::from_bytes(bytes)?),
            0x46 => Self::GetHue(GetHueNotify::from_bytes(bytes)?),
            0x47 => Self::SetHue(ErrorResultNotify::<0x47>::from_bytes(bytes)?),
            0x48 => Self::GetSaturation(GetSaturationNotify::from_bytes(bytes)?),
            0x49 => Self::SetSaturation(ErrorResultNotify::<0x49>::from_bytes(bytes)?),
            0x4E => Self::GetSharpness(GetSharpnessNotify::from_bytes(bytes)?),
            0x4F => Self::SetSharpness(ErrorResultNotify::<0x4F>::from_bytes(bytes)?),
            0x82 => Self::UpdateFwImage(UpdateResultNotify::<0x82>::from_bytes(bytes)?),
            0x84 => Self::UpdatePictureData(UpdateResultNotify::<0x84>::from_bytes(bytes)?),
            0x92 => Self::DivisionTransmissionUpdateFwImage(
                UpdateResultNotify::<0x92>::from_bytes(bytes)?,
            ),
            0x94 => Self::DivisionTransmissionUpdatePictureData(
                UpdateResultNotify::<0x94>::from_bytes(bytes)?,
            ),
            0xA0 => Self::GetTemperature(GetTemperatureNotify::from_bytes(bytes)?),
            0xA1 => Self::GetTime(GetTimeNotify::from_bytes(bytes)?),
            0xA2 => Self::GetVersion(GetVersionNotify::from_bytes(bytes)?),
            0xA3 => Self::OutputTestPicture(ErrorResultNotify::<0xA3>::from_bytes(bytes)?),
            0xB2 => Self::GetLotNumber(GetLotNumberNotify::from_bytes(bytes)?),
            0xB4 => Self::GetSerialNumber(GetSerialNumberNotify::from_bytes(bytes)?),
            _ => {
                return Err(CommandError::WrongCommand {
                    expected: 0,
                    actual: cmd,
                })
            }
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StartInput;
empty_request!(StartInput, 0x01);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StopInput;
empty_request!(StopInput, 0x02);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopInputSpecially {
    pub output: StoppedOutput,
}
one_byte_request!(StopInputSpecially, 0x0C, output);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MuteUnmuteChangeOutput {
    pub output: MuteOutput,
}
one_byte_request!(MuteUnmuteChangeOutput, 0x03, output);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaveUserParam {
    pub video_position: SaveVideoPosition,
    pub optical_alignment_biphase: SaveOpticalAlignmentBiphase,
    pub picture_quality: SavePictureQuality,
}

impl FixedSizeRequest for SaveUserParam {
    const CMD: u8 = 0x07;

    fn ops(&self) -> Vec<u8> {
        vec![
            0x00,
            0x00,
            self.video_position.into(),
            self.optical_alignment_biphase.into(),
            self.picture_quality.into(),
        ]
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InitializeUserParam;
empty_request!(InitializeUserParam, 0x08);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShutdownReboot {
    pub option: ShutdownOption,
}
one_byte_request!(ShutdownReboot, 0x0B, option);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetVideoOutputPosition;
empty_request!(GetVideoOutputPosition, 0x25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetVideoOutputPosition {
    pub pan: Pan,
    pub tilt: Tilt,
    pub flip: Flip,
}

impl FixedSizeRequest for SetVideoOutputPosition {
    const CMD: u8 = 0x26;

    fn ops(&self) -> Vec<u8> {
        vec![
            self.pan.into(),
            self.tilt.into(),
            self.flip.into(),
            0x64,
            0x00,
            0x00,
            0x00,
            0x00,
            0x00,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetOpticalAlignment;
empty_request!(GetOpticalAlignment, 0x27);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpticalAlignment {
    pub r0_horizontal: I8Byte,
    pub r1_horizontal: I8Byte,
    pub g0_horizontal: I8Byte,
    pub g1_horizontal: I8Byte,
    pub b_horizontal: I8Byte,
    pub r0_vertical: u8,
    pub r1_vertical: u8,
    pub g0_vertical: u8,
    pub g1_vertical: u8,
    pub b_vertical: u8,
}

impl OpticalAlignment {
    pub fn ops(self) -> [u8; 13] {
        [
            self.r0_horizontal.into(),
            self.r1_horizontal.into(),
            self.g0_horizontal.into(),
            self.g1_horizontal.into(),
            self.b_horizontal.into(),
            self.r0_vertical,
            self.r1_vertical,
            self.g0_vertical,
            self.g1_vertical,
            self.b_vertical,
            0x00,
            0x00,
            0x00,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetOpticalAlignment {
    pub alignment: OpticalAlignment,
}

impl FixedSizeRequest for SetOpticalAlignment {
    const CMD: u8 = 0x28;

    fn ops(&self) -> Vec<u8> {
        self.alignment.ops().to_vec()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetBiphase;
empty_request!(GetBiphase, 0x29);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetBiphase {
    pub amount: u32,
}

impl FixedSizeRequest for SetBiphase {
    const CMD: u8 = 0x2A;

    fn ops(&self) -> Vec<u8> {
        self.amount.to_le_bytes().to_vec()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyOpticalAdjustmentControl;
empty_request!(SetEasyOpticalAdjustmentControl, 0x32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyOpticalAdjustmentPlus;
empty_request!(SetEasyOpticalAdjustmentPlus, 0x33);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyOpticalAdjustmentMinus;
empty_request!(SetEasyOpticalAdjustmentMinus, 0x34);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyOpticalAdjustmentExit {
    pub save: SaveAdjustment,
}
one_byte_request!(SetEasyOpticalAdjustmentExit, 0x35, save);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyBiphaseAdjustmentControl;
empty_request!(SetEasyBiphaseAdjustmentControl, 0x36);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyBiphaseAdjustmentPlus;
empty_request!(SetEasyBiphaseAdjustmentPlus, 0x37);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyBiphaseAdjustmentMinus;
empty_request!(SetEasyBiphaseAdjustmentMinus, 0x38);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetEasyBiphaseAdjustmentExit {
    pub save: SaveAdjustment,
}
one_byte_request!(SetEasyBiphaseAdjustmentExit, 0x39, save);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetAllPictureQuality;
empty_request!(GetAllPictureQuality, 0x40);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PictureQuality {
    pub contrast: i8,
    pub brightness: i8,
    pub hue_u: i8,
    pub hue_v: i8,
    pub saturation_u: i8,
    pub saturation_v: i8,
    pub sharpness: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetAllPictureQuality {
    pub picture_quality: PictureQuality,
}

impl FixedSizeRequest for SetAllPictureQuality {
    const CMD: u8 = 0x41;

    fn ops(&self) -> Vec<u8> {
        vec![
            self.picture_quality.contrast as u8,
            self.picture_quality.brightness as u8,
            self.picture_quality.hue_u as u8,
            self.picture_quality.hue_v as u8,
            self.picture_quality.saturation_u as u8,
            self.picture_quality.saturation_v as u8,
            0x00,
            self.picture_quality.sharpness,
            0x00,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetBrightness;
empty_request!(GetBrightness, 0x42);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetBrightness {
    pub brightness: i8,
}
i8_request!(SetBrightness, 0x43, brightness);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetContrast;
empty_request!(GetContrast, 0x44);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetContrast {
    pub contrast: i8,
}
i8_request!(SetContrast, 0x45, contrast);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetHue;
empty_request!(GetHue, 0x46);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetHue {
    pub hue_u: i8,
    pub hue_v: i8,
}

impl FixedSizeRequest for SetHue {
    const CMD: u8 = 0x47;

    fn ops(&self) -> Vec<u8> {
        vec![self.hue_u as u8, self.hue_v as u8]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetSaturation;
empty_request!(GetSaturation, 0x48);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetSaturation {
    pub saturation_u: i8,
    pub saturation_v: i8,
}

impl FixedSizeRequest for SetSaturation {
    const CMD: u8 = 0x49;

    fn ops(&self) -> Vec<u8> {
        vec![self.saturation_u as u8, self.saturation_v as u8]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetSharpness;
empty_request!(GetSharpness, 0x4E);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetSharpness {
    pub sharpness: u8,
}

impl FixedSizeRequest for SetSharpness {
    const CMD: u8 = 0x4F;

    fn ops(&self) -> Vec<u8> {
        vec![self.sharpness]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateFwImage {
    pub data_size: u32,
    pub checksum: u32,
    pub data: Vec<u8>,
}
update_request!(UpdateFwImage, 0x82);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdatePictureData {
    pub data_size: u32,
    pub checksum: u32,
    pub data: Vec<u8>,
}
update_request!(UpdatePictureData, 0x84);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivisionTransmissionUpdateFwImage {
    pub data_size: u32,
    pub checksum: u32,
    pub format: DivisionTransmissionFormat,
    pub max_block_number: u16,
}
division_start_request!(DivisionTransmissionUpdateFwImage, 0x92);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivisionTransmissionUpdatePictureData {
    pub data_size: u32,
    pub checksum: u32,
    pub format: DivisionTransmissionFormat,
    pub max_block_number: u16,
}
division_start_request!(DivisionTransmissionUpdatePictureData, 0x94);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivisionTransmissionUpdateData {
    pub format: DivisionTransmissionFormat,
    pub block_number: u16,
    pub data: Vec<u8>,
}

impl DivisionTransmissionUpdateData {
    pub const CMD: u8 = 0x9F;

    pub fn new(
        format: DivisionTransmissionFormat,
        block_number: u16,
        data: impl Into<Vec<u8>>,
    ) -> Result<Self> {
        let data = data.into();
        let max = format.block_size();
        if data.len() > max {
            return Err(CommandError::DivisionBlockTooLarge {
                max,
                actual: data.len(),
            });
        }
        Ok(Self {
            format,
            block_number,
            data,
        })
    }
}

impl Request for DivisionTransmissionUpdateData {
    fn command_id(&self) -> u8 {
        Self::CMD
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.format.block_size() + 4);
        bytes.push(Self::CMD);
        bytes.push(self.format.into());
        bytes.extend_from_slice(&self.block_number.to_le_bytes());
        bytes.extend_from_slice(&self.data);
        bytes.resize(self.format.block_size() + 4, 0x00);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetTemperature;
empty_request!(GetTemperature, 0xA0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetTime;
empty_request!(GetTime, 0xA1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetVersion;
empty_request!(GetVersion, 0xA2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputTestPicture {
    pub pattern: TestPattern,
    pub setting: u8,
    pub background_color: Rgb,
    pub foreground_color: Rgb,
}

impl FixedSizeRequest for OutputTestPicture {
    const CMD: u8 = 0xA3;

    fn ops(&self) -> Vec<u8> {
        let mut ops = vec![
            self.pattern.into(),
            self.setting,
            self.background_color.r,
            self.background_color.g,
            self.background_color.b,
            self.foreground_color.r,
            self.foreground_color.g,
            self.foreground_color.b,
        ];
        ops.resize(17, 0x00);
        ops
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetLotNumber;
empty_request!(GetLotNumber, 0xB2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetSerialNumber;
empty_request!(GetSerialNumber, 0xB4);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootCompletedNotify {
    pub result: BootResult,
}

impl Notify for BootCompletedNotify {
    const CMD: u8 = 0x00;
    const SIZE: usize = 1;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: BootResult::from(ops[0]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandResultNotify<const CMD: u8> {
    pub result: CommandResult,
}

impl<const CMD: u8> Notify for CommandResultNotify<CMD> {
    const CMD: u8 = CMD;
    const SIZE: usize = 1;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorResultNotify<const CMD: u8> {
    pub result: u8,
}

impl<const CMD: u8> Notify for ErrorResultNotify<CMD> {
    const CMD: u8 = CMD;
    const SIZE: usize = 1;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self { result: ops[0] })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpdateResultNotify<const CMD: u8> {
    pub result: UpdateResult,
}

impl<const CMD: u8> Notify for UpdateResultNotify<CMD> {
    const CMD: u8 = CMD;
    const SIZE: usize = 1;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: UpdateResult::from(ops[0]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjustmentControlNotify<const CMD: u8> {
    pub result: AdjustmentControlResult,
}

impl<const CMD: u8> Notify for AdjustmentControlNotify<CMD> {
    const CMD: u8 = CMD;
    const SIZE: usize = 1;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: AdjustmentControlResult::from(ops[0]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdjustmentStepNotify<const CMD: u8> {
    pub result: AdjustmentStepResult,
}

impl<const CMD: u8> Notify for AdjustmentStepNotify<CMD> {
    const CMD: u8 = CMD;
    const SIZE: usize = 1;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: AdjustmentStepResult::from(ops[0]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetVideoOutputPositionNotify {
    pub result: CommandResult,
    pub pan: Pan,
    pub tilt: Tilt,
    pub flip: Flip,
    pub reserved: [u8; 6],
}

impl Notify for GetVideoOutputPositionNotify {
    const CMD: u8 = 0x25;
    const SIZE: usize = 10;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            pan: Pan(ops[1] as i8),
            tilt: Tilt(ops[2] as i8),
            flip: Flip::try_from(ops[3])?,
            reserved: ops[4..10].try_into().unwrap(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetOpticalAlignmentNotify {
    pub result: CommandResult,
    pub alignment: OpticalAlignment,
    pub reserved: [u8; 3],
}

impl Notify for GetOpticalAlignmentNotify {
    const CMD: u8 = 0x27;
    const SIZE: usize = 14;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            alignment: OpticalAlignment {
                r0_horizontal: I8Byte(ops[1] as i8),
                r1_horizontal: I8Byte(ops[2] as i8),
                g0_horizontal: I8Byte(ops[3] as i8),
                g1_horizontal: I8Byte(ops[4] as i8),
                b_horizontal: I8Byte(ops[5] as i8),
                r0_vertical: ops[6],
                r1_vertical: ops[7],
                g0_vertical: ops[8],
                g1_vertical: ops[9],
                b_vertical: ops[10],
            },
            reserved: ops[11..14].try_into().unwrap(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetBiphaseNotify {
    pub result: CommandResult,
    pub amount: u32,
}

impl Notify for GetBiphaseNotify {
    const CMD: u8 = 0x29;
    const SIZE: usize = 5;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            amount: u32::from_le_bytes(ops[1..5].try_into().unwrap()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetAllPictureQualityNotify {
    pub result: CommandResult,
    pub picture_quality: PictureQuality,
    pub reserved1: u8,
    pub reserved2: u8,
}

impl Notify for GetAllPictureQualityNotify {
    const CMD: u8 = 0x40;
    const SIZE: usize = 10;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            picture_quality: PictureQuality {
                contrast: ops[1] as i8,
                brightness: ops[2] as i8,
                hue_u: ops[3] as i8,
                hue_v: ops[4] as i8,
                saturation_u: ops[5] as i8,
                saturation_v: ops[6] as i8,
                sharpness: ops[8],
            },
            reserved1: ops[7],
            reserved2: ops[9],
        })
    }
}

scalar_notify!(GetBrightnessNotify, 0x42, brightness, i8);
scalar_notify!(GetContrastNotify, 0x44, contrast, i8);
pair_notify!(GetHueNotify, 0x46, hue_u, hue_v, i8);
pair_notify!(GetSaturationNotify, 0x48, saturation_u, saturation_v, i8);
scalar_notify!(GetSharpnessNotify, 0x4E, sharpness, u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetTemperatureNotify {
    pub result: CommandResult,
    pub module_temperature: u8,
    pub mute_threshold_temperature: u8,
    pub system_stop_threshold_temperature: u8,
}

impl Notify for GetTemperatureNotify {
    const CMD: u8 = 0xA0;
    const SIZE: usize = 4;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            module_temperature: ops[1],
            mute_threshold_temperature: ops[2],
            system_stop_threshold_temperature: ops[3],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetTimeNotify {
    pub result: CommandResult,
    pub seconds: u32,
}

impl Notify for GetTimeNotify {
    const CMD: u8 = 0xA1;
    const SIZE: usize = 5;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            seconds: u32::from_le_bytes(ops[1..5].try_into().unwrap()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetVersionNotify {
    pub result: CommandResult,
    pub firmware: [u8; 4],
    pub parameter: [u8; 4],
    pub data: [u8; 4],
}

impl Notify for GetVersionNotify {
    const CMD: u8 = 0xA2;
    const SIZE: usize = 13;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            firmware: ops[1..5].try_into().unwrap(),
            parameter: ops[5..9].try_into().unwrap(),
            data: ops[9..13].try_into().unwrap(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetLotNumberNotify {
    pub result: CommandResult,
    pub lot_numbers: [u32; 3],
}

impl Notify for GetLotNumberNotify {
    const CMD: u8 = 0xB2;
    const SIZE: usize = 13;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            lot_numbers: [
                u32::from_le_bytes(ops[1..5].try_into().unwrap()),
                u32::from_le_bytes(ops[5..9].try_into().unwrap()),
                u32::from_le_bytes(ops[9..13].try_into().unwrap()),
            ],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetSerialNumberNotify {
    pub result: CommandResult,
    pub serial_numbers: [u32; 2],
}

impl Notify for GetSerialNumberNotify {
    const CMD: u8 = 0xB4;
    const SIZE: usize = 9;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandResult::from(ops[0]),
            serial_numbers: [
                u32::from_le_bytes(ops[1..5].try_into().unwrap()),
                u32::from_le_bytes(ops[5..9].try_into().unwrap()),
            ],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmergencyNotify {
    pub result: EmergencyResult,
    pub reserved: [u8; 2],
}

impl Notify for EmergencyNotify {
    const CMD: u8 = 0x10;
    const SIZE: usize = 3;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: EmergencyResult::from(ops[0]),
            reserved: [ops[1], ops[2]],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemperatureEmergencyAndRecoveryNotify {
    pub result: TemperatureEmergencyRecoveryResult,
}

impl Notify for TemperatureEmergencyAndRecoveryNotify {
    const CMD: u8 = 0x11;
    const SIZE: usize = 1;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: TemperatureEmergencyRecoveryResult::from(ops[0]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandEmergencyNotify {
    pub result: CommandEmergencyResult,
    pub reference: u8,
}

impl Notify for CommandEmergencyNotify {
    const CMD: u8 = 0x12;
    const SIZE: usize = 2;

    fn from_ops(ops: &[u8]) -> Result<Self> {
        Ok(Self {
            result: CommandEmergencyResult::from(ops[0]),
            reference: ops[1],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I8Byte(pub i8);

impl From<I8Byte> for u8 {
    fn from(value: I8Byte) -> Self {
        value.0 as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pan(pub i8);

impl From<Pan> for u8 {
    fn from(value: Pan) -> Self {
        value.0 as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tilt(pub i8);

impl From<Tilt> for u8 {
    fn from(value: Tilt) -> Self {
        value.0 as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootResult {
    Normal,
    InternalMalfunction,
    InternalFailure(u8),
    ParameterTransactionFailure,
    Unknown(u8),
}

impl From<u8> for BootResult {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Normal,
            0x80 => Self::InternalMalfunction,
            0x81..=0x84 => Self::InternalFailure(value & 0x0F),
            0xFE => Self::ParameterTransactionFailure,
            _ => Self::Unknown(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandResult {
    Normal,
    Abnormal(u8),
    Other(u8),
}

impl From<u8> for CommandResult {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Normal,
            0x80..=0x8F => Self::Abnormal(value),
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentControlResult {
    Executed,
    Completed,
    Abnormal(u8),
    Other(u8),
}

impl From<u8> for AdjustmentControlResult {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Executed,
            0x01 => Self::Completed,
            0x80..=0x8F => Self::Abnormal(value),
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentStepResult {
    Normal,
    Abnormal(u8),
    LimitExceeded,
    Other(u8),
}

impl From<u8> for AdjustmentStepResult {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Normal,
            0x80..=0x8F => Self::Abnormal(value),
            0xFE => Self::LimitExceeded,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateResult {
    Normal,
    CmdFormatError,
    TimeoutError,
    ChecksumError,
    NumberError,
    OtherError,
    Other(u8),
}

impl From<u8> for UpdateResult {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Normal,
            0xF0 => Self::CmdFormatError,
            0xF1 => Self::TimeoutError,
            0xF2 => Self::ChecksumError,
            0xF3 => Self::NumberError,
            0xF4 => Self::OtherError,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyResult {
    LaserSafetyModuleAbnormality,
    FirmwareAbort,
    MemsLaserAbnormality,
    UnderflowRecovered,
    Other(u8),
}

impl From<u8> for EmergencyResult {
    fn from(value: u8) -> Self {
        match value {
            0x80 => Self::LaserSafetyModuleAbnormality,
            0x81 => Self::FirmwareAbort,
            0x82 => Self::MemsLaserAbnormality,
            0x83 => Self::UnderflowRecovered,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemperatureEmergencyRecoveryResult {
    MuteThresholdExceeded,
    SystemThresholdExceeded,
    RecoveredFromMuteThreshold,
    RecoveredFromSystemThreshold,
    Other(u8),
}

impl From<u8> for TemperatureEmergencyRecoveryResult {
    fn from(value: u8) -> Self {
        match value {
            0x80 => Self::MuteThresholdExceeded,
            0x81 => Self::SystemThresholdExceeded,
            0x00 => Self::RecoveredFromMuteThreshold,
            0x01 => Self::RecoveredFromSystemThreshold,
            _ => Self::Other(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEmergencyResult {
    TooManyCommands,
    ReceptionTimeout,
    Other(u8),
}

impl From<u8> for CommandEmergencyResult {
    fn from(value: u8) -> Self {
        match value {
            0xFB => Self::TooManyCommands,
            0xF0 => Self::ReceptionTimeout,
            _ => Self::Other(value),
        }
    }
}

byte_enum!(StoppedOutput {
    OpeningPicture = 0x00,
    MuteBlackPicture = 0x01,
    MuteOpeningPicture = 0x02,
    FinalPicture = 0x04,
});

byte_enum!(MuteOutput {
    Unmute = 0x00,
    MuteBlackPicture = 0x01,
    MuteOpeningPicture = 0x02,
    FinalPicture = 0x04,
});

byte_enum!(SaveVideoPosition {
    DontSave = 0x00,
    All = 0x01,
    FlipOnly = 0x02,
});

byte_enum!(SaveOpticalAlignmentBiphase {
    DontSave = 0x00,
    All = 0x01,
    OpticalAlignmentOnly = 0x02,
    BiphaseOnly = 0x03,
});

byte_enum!(SavePictureQuality {
    DontSave = 0x00,
    Save = 0x01,
});

byte_enum!(ShutdownOption {
    StopsAllFunctions = 0x00,
    Reboot = 0x01,
});

byte_enum!(Flip {
    Off = 0x00,
    RightLeft = 0x01,
    UpDown = 0x02,
    UpDownAndRightLeft = 0x03,
});

byte_enum!(SaveAdjustment {
    DontSave = 0x00,
    Save = 0x01,
});

byte_enum!(DivisionTransmissionFormat {
    Size64 = 0x00,
    Size256 = 0x01,
    Size1K = 0x02,
    Size4K = 0x03,
    Size16K = 0x04,
});

impl DivisionTransmissionFormat {
    pub const fn block_size(self) -> usize {
        match self {
            Self::Size64 => 64,
            Self::Size256 => 256,
            Self::Size1K => 1024,
            Self::Size4K => 4096,
            Self::Size16K => 16 * 1024,
        }
    }
}

byte_enum!(TestPattern {
    Stop = 0x00,
    ColorBar = 0x01,
    CrossHatching = 0x02,
    Raster = 0x03,
    Ramp = 0x04,
    Circle = 0x05,
    Cross = 0x06,
    CircleAndCross = 0x07,
    FilledCircle = 0x08,
    FilledSquare = 0x09,
    Checkerboard = 0x0A,
    ResolutionCheckerVerticalLines = 0x0B,
    ResolutionCheckerHorizontalLines = 0x0C,
    ResolutionCheckerSquare = 0x0D,
    ColorBarRampSpecial = 0x0E,
    RectangularHatchEven = 0x0F,
    RectangularHatchEqualCrossHatchInterval = 0x10,
});

fn command_with_sized_ops(cmd: u8, ops: Vec<u8>) -> Vec<u8> {
    assert!(
        ops.len() <= u8::MAX as usize,
        "CXN0102 OP payload too large"
    );
    let mut bytes = Vec::with_capacity(ops.len() + 2);
    bytes.push(cmd);
    bytes.push(ops.len() as u8);
    bytes.extend_from_slice(&ops);
    bytes
}

fn update_bytes(cmd: u8, data_size: u32, checksum: u32, data: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(data.len() + 9);
    bytes.push(cmd);
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.extend_from_slice(&checksum.to_le_bytes());
    bytes.extend_from_slice(data);
    bytes
}

fn division_start_bytes(
    cmd: u8,
    data_size: u32,
    checksum: u32,
    format: DivisionTransmissionFormat,
    max_block_number: u16,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(12);
    bytes.push(cmd);
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.extend_from_slice(&checksum.to_le_bytes());
    bytes.push(format.into());
    bytes.extend_from_slice(&max_block_number.to_le_bytes());
    bytes
}

macro_rules! empty_request {
    ($name:ident, $cmd:expr) => {
        impl FixedSizeRequest for $name {
            const CMD: u8 = $cmd;

            fn ops(&self) -> Vec<u8> {
                Vec::new()
            }
        }
    };
}

macro_rules! one_byte_request {
    ($name:ident, $cmd:expr, $field:ident) => {
        impl FixedSizeRequest for $name {
            const CMD: u8 = $cmd;

            fn ops(&self) -> Vec<u8> {
                vec![self.$field.into()]
            }
        }
    };
}

macro_rules! i8_request {
    ($name:ident, $cmd:expr, $field:ident) => {
        impl FixedSizeRequest for $name {
            const CMD: u8 = $cmd;

            fn ops(&self) -> Vec<u8> {
                vec![self.$field as u8]
            }
        }
    };
}

macro_rules! update_request {
    ($name:ident, $cmd:expr) => {
        impl Request for $name {
            fn command_id(&self) -> u8 {
                $cmd
            }

            fn to_bytes(&self) -> Vec<u8> {
                update_bytes($cmd, self.data_size, self.checksum, &self.data)
            }
        }
    };
}

macro_rules! division_start_request {
    ($name:ident, $cmd:expr) => {
        impl Request for $name {
            fn command_id(&self) -> u8 {
                $cmd
            }

            fn to_bytes(&self) -> Vec<u8> {
                division_start_bytes(
                    $cmd,
                    self.data_size,
                    self.checksum,
                    self.format,
                    self.max_block_number,
                )
            }
        }
    };
}

macro_rules! scalar_notify {
    ($name:ident, $cmd:expr, $field:ident, i8) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            pub result: CommandResult,
            pub $field: i8,
        }

        impl Notify for $name {
            const CMD: u8 = $cmd;
            const SIZE: usize = 2;

            fn from_ops(ops: &[u8]) -> Result<Self> {
                Ok(Self {
                    result: CommandResult::from(ops[0]),
                    $field: ops[1] as i8,
                })
            }
        }
    };
    ($name:ident, $cmd:expr, $field:ident, u8) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            pub result: CommandResult,
            pub $field: u8,
        }

        impl Notify for $name {
            const CMD: u8 = $cmd;
            const SIZE: usize = 2;

            fn from_ops(ops: &[u8]) -> Result<Self> {
                Ok(Self {
                    result: CommandResult::from(ops[0]),
                    $field: ops[1],
                })
            }
        }
    };
}

macro_rules! pair_notify {
    ($name:ident, $cmd:expr, $field1:ident, $field2:ident, i8) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct $name {
            pub result: CommandResult,
            pub $field1: i8,
            pub $field2: i8,
        }

        impl Notify for $name {
            const CMD: u8 = $cmd;
            const SIZE: usize = 3;

            fn from_ops(ops: &[u8]) -> Result<Self> {
                Ok(Self {
                    result: CommandResult::from(ops[0]),
                    $field1: ops[1] as i8,
                    $field2: ops[2] as i8,
                })
            }
        }
    };
}

macro_rules! byte_enum {
    ($name:ident { $($variant:ident = $value:expr,)+ }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($variant,)+
        }

        impl From<$name> for u8 {
            fn from(value: $name) -> Self {
                match value {
                    $($name::$variant => $value,)+
                }
            }
        }

        impl TryFrom<u8> for $name {
            type Error = CommandError;

            fn try_from(value: u8) -> Result<Self> {
                match value {
                    $($value => Ok($name::$variant),)+
                    _ => Err(CommandError::InvalidValue {
                        field: stringify!($name),
                        value,
                    }),
                }
            }
        }
    };
}

pub(crate) use byte_enum;
pub(crate) use division_start_request;
pub(crate) use empty_request;
pub(crate) use i8_request;
pub(crate) use one_byte_request;
pub(crate) use pair_notify;
pub(crate) use scalar_notify;
pub(crate) use update_request;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_reboot_serializes_with_size_op0() {
        let request = ShutdownReboot {
            option: ShutdownOption::StopsAllFunctions,
        };

        assert_eq!(request.to_bytes(), [0x0B, 0x01, 0x00]);
    }

    #[test]
    fn set_video_output_position_serializes_fixed_values() {
        let request = SetVideoOutputPosition {
            pan: Pan(-30),
            tilt: Tilt(20),
            flip: Flip::UpDownAndRightLeft,
        };

        assert_eq!(
            request.to_bytes(),
            [0x26, 0x09, 0xE2, 0x14, 0x03, 0x64, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn special_update_serializes_without_size_op0() {
        let request = UpdateFwImage {
            data_size: 0x11223344,
            checksum: 0x55667788,
            data: vec![0xAA, 0xBB],
        };

        assert_eq!(
            request.to_bytes(),
            [0x82, 0x44, 0x33, 0x22, 0x11, 0x88, 0x77, 0x66, 0x55, 0xAA, 0xBB]
        );
    }

    #[test]
    fn parses_get_time_notify() {
        let notify =
            GetTimeNotify::from_bytes(&[0xA1, 0x05, 0x00, 0x44, 0x33, 0x22, 0x11]).unwrap();

        assert_eq!(notify.result, CommandResult::Normal);
        assert_eq!(notify.seconds, 0x11223344);
    }
}