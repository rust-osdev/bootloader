use {
    acpi::{
        platform::{
            address::GenericAddress,
            interrupt::{
                Apic, InterruptSourceOverride, IoApic, LocalInterruptLine, NmiLine, NmiProcessor,
                NmiSource, Polarity, TriggerMode,
            },
            PlatformInfo, PmTimer, Processor, ProcessorInfo,
        },
        sdt::Signature,
        AcpiHandler, AcpiTables, AmlTable,
        InterruptModel::{self, *},
        PowerProfile, Sdt,
    },
    alloc::vec::Vec,
};

/// Drop-in replacement for `acpi::platform::interrupt::Apic` that uses slices instead of vectors
/// for easy transportation across the UEFI Boot Services boundary
#[derive(Debug)]
pub struct PortableApic {
    pub local_apic_address: u64,
    pub io_apics: &'static mut [IoApic],
    pub local_apic_nmi_lines: &'static mut [NmiLine],
    pub interrupt_source_overrides: &'static mut [InterruptSourceOverride],
    pub nmi_sources: &'static mut [NmiSource],
    pub also_has_legacy_pics: bool,
}

impl PortableApic {
    pub fn new(upstream: Apic) -> Self {
        Self {
            local_apic_address: upstream.local_apic_address,
            io_apics: upstream.io_apics.leak(),
            local_apic_nmi_lines: upstream.local_apic_nmi_lines.leak(),
            interrupt_source_overrides: upstream.interrupt_source_overrides.leak(),
            nmi_sources: upstream.nmi_sources.leak(),
            also_has_legacy_pics: upstream.also_has_legacy_pics,
        }
    }
}

impl Clone for PortableApic {
    fn clone(&self) -> Self {
        // Rebuild the IOAPIC vector
        let new_io_apic_vec = self
            .io_apics
            .iter()
            .map(|apic| IoApic {
                id: apic.id,
                address: apic.address,
                global_system_interrupt_base: apic.global_system_interrupt_base,
            })
            .collect::<Vec<_>>();

        // Rebuild the NMI lines
        let new_nmi_line_vec = self
            .local_apic_nmi_lines
            .iter()
            .map(|nmi_line| {
                let processor = match nmi_line.processor {
                    NmiProcessor::All => NmiProcessor::All,
                    NmiProcessor::ProcessorUid(t) => NmiProcessor::ProcessorUid(t),
                };
                let line = match nmi_line.line {
                    LocalInterruptLine::Lint0 => LocalInterruptLine::Lint0,
                    LocalInterruptLine::Lint1 => LocalInterruptLine::Lint1,
                };
                NmiLine { processor, line }
            })
            .collect::<Vec<_>>();

        // Rebuild the source overrides
        let new_source_override_vec = self
            .interrupt_source_overrides
            .iter()
            .map(|src_override| {
                let polarity = match src_override.polarity {
                    Polarity::SameAsBus => Polarity::SameAsBus,
                    Polarity::ActiveHigh => Polarity::ActiveHigh,
                    Polarity::ActiveLow => Polarity::ActiveLow,
                };
                let trigger_mode = match src_override.trigger_mode {
                    TriggerMode::SameAsBus => TriggerMode::SameAsBus,
                    TriggerMode::Edge => TriggerMode::Edge,
                    TriggerMode::Level => TriggerMode::Level,
                };
                InterruptSourceOverride {
                    isa_source: src_override.isa_source,
                    global_system_interrupt: src_override.global_system_interrupt,
                    polarity,
                    trigger_mode,
                }
            })
            .collect::<Vec<_>>();

        // Rebuild the NMI sources
        let new_nmi_source_vec = self
            .nmi_sources
            .iter()
            .map(|source| {
                let polarity = match source.polarity {
                    Polarity::SameAsBus => Polarity::SameAsBus,
                    Polarity::ActiveHigh => Polarity::ActiveHigh,
                    Polarity::ActiveLow => Polarity::ActiveLow,
                };
                let trigger_mode = match source.trigger_mode {
                    TriggerMode::SameAsBus => TriggerMode::SameAsBus,
                    TriggerMode::Edge => TriggerMode::Edge,
                    TriggerMode::Level => TriggerMode::Level,
                };
                NmiSource {
                    global_system_interrupt: source.global_system_interrupt,
                    polarity,
                    trigger_mode,
                }
            })
            .collect::<Vec<_>>();

        // Take everything that's been rebuilt from scratch and leak it all over again
        Self {
            local_apic_address: self.local_apic_address.clone(),
            io_apics: new_io_apic_vec.leak(),
            local_apic_nmi_lines: new_nmi_line_vec.leak(),
            interrupt_source_overrides: new_source_override_vec.leak(),
            nmi_sources: new_nmi_source_vec.leak(),
            also_has_legacy_pics: self.also_has_legacy_pics.clone(),
        }
    }
}

// Allow globals
unsafe impl Send for PortableApic {}
unsafe impl Sync for PortableApic {}

/// Drop-in replacement for `acpi::InterruptModel` that uses slices instead of vectors
/// for easy transportation across the UEFI Boot Services boundary
#[derive(Debug, Clone)]
pub enum PortableInterruptModel {
    Unknown,
    Apic(PortableApic),
}

impl PortableInterruptModel {
    pub fn new(upstream: InterruptModel) -> Self {
        match upstream {
            Unknown => Self::Unknown,
            Apic(apic) => Self::Apic(PortableApic::new(apic)),
            _ => unreachable!(), // compiler throws an error if this arm isn't included
        }
    }
}

// Allow globals
unsafe impl Send for PortableInterruptModel {}
unsafe impl Sync for PortableInterruptModel {}

/// Drop-in replacement for `acpi::platform::ProcessorInfo` that uses slices instead of vectors
/// for easy transportation across the UEFI Boot Services boundary
pub struct PortableProcessorInfo {
    pub boot_processor: Processor,
    pub app_processors: &'static mut [Processor],
}

impl PortableProcessorInfo {
    pub fn new(upstream: ProcessorInfo) -> Self {
        Self {
            boot_processor: upstream.boot_processor,
            app_processors: upstream.application_processors.leak(),
        }
    }
}

impl Clone for PortableProcessorInfo {
    fn clone(&self) -> Self {
        let new_app_proc_vec = self
            .app_processors
            .iter()
            .map(|i| i.clone())
            .collect::<Vec<_>>();
        Self {
            boot_processor: self.boot_processor.clone(),
            app_processors: new_app_proc_vec.leak(),
        }
    }
}

// Allow globals
unsafe impl Send for PortableProcessorInfo {}
unsafe impl Sync for PortableProcessorInfo {}

/// Like `acpi::platform::PmTimer` except `Clone`
#[derive(Clone)]
pub struct ClonePmTimer {
    pub base: GenericAddress,
    pub supports_32bit: bool,
}

impl ClonePmTimer {
    pub fn new(upstream: PmTimer) -> Self {
        Self {
            base: upstream.base,
            supports_32bit: upstream.supports_32bit,
        }
    }
}

// Allow globals
unsafe impl Send for ClonePmTimer {}
unsafe impl Sync for ClonePmTimer {}

/// Drop-in replacement for `acpi::platform::PlatformInfo` that uses slices instead of vectors
/// for easy transportation across the UEFI Boot Services boundary
#[derive(Clone)]
pub struct PortablePlatformInfo {
    pub power: PowerProfile,
    pub interrupt: PortableInterruptModel,
    pub processor_info: Option<PortableProcessorInfo>,
    pub pm_timer: Option<ClonePmTimer>,
}

impl PortablePlatformInfo {
    pub fn new(upstream: PlatformInfo) -> Self {
        let new_proc_info = match upstream.processor_info {
            Some(info) => Some(PortableProcessorInfo::new(ProcessorInfo {
                boot_processor: info.boot_processor.clone(),
                application_processors: info.application_processors.clone(),
            })),
            None => None,
        };

        let new_pm_timer = match upstream.pm_timer {
            Some(timer) => Some(ClonePmTimer::new(PmTimer {
                base: timer.base.clone(),
                supports_32bit: timer.supports_32bit.clone(),
            })),
            None => None,
        };

        Self {
            power: upstream.power_profile,
            interrupt: PortableInterruptModel::new(upstream.interrupt_model),
            processor_info: new_proc_info,
            pm_timer: new_pm_timer,
        }
    }
}

// Allow globals
unsafe impl Send for PortablePlatformInfo {}
unsafe impl Sync for PortablePlatformInfo {}

/// Drop-in replacement for `acpi::AcpiTables` that uses slices instead of vectors and BTreeMaps
/// for easy transportation across the UEFI Boot Services boundary
pub struct PortableAcpiTables {
    pub revision: u8,
    pub sdts: &'static mut [(Signature, Sdt)],
    pub dsdt: Option<AmlTable>,
    pub ssdts: &'static mut [AmlTable],
    pub info: PortablePlatformInfo,
}

impl PortableAcpiTables {
    pub fn new<H: AcpiHandler>(upstream: AcpiTables<H>) -> Self {
        // need to borrow way up here to satisfy the borrow checker
        let new_platform_info = PlatformInfo::new(&upstream).unwrap();

        // rebuild structures
        let sdt_vec = upstream.sdts.into_iter().collect::<Vec<_>>();
        let sdt_slice = sdt_vec.leak();

        let copied_aml_vec = upstream
            .ssdts
            .into_iter()
            .map(|table| AmlTable {
                address: table.address,
                length: table.length,
            })
            .collect::<Vec<_>>();

        let copied_dsdt = match upstream.dsdt {
            Some(table) => Some(AmlTable {
                address: table.address,
                length: table.length,
            }),
            None => None,
        };

        Self {
            revision: upstream.revision,
            sdts: sdt_slice,
            dsdt: copied_dsdt,
            ssdts: copied_aml_vec.leak(),
            info: PortablePlatformInfo::new(new_platform_info),
        }
    }
}

impl Clone for PortableAcpiTables {
    fn clone(&self) -> Self {
        // as before: recreate SDT map
        let new_sdt_vec = self
            .sdts
            .iter()
            .map(|sdt| {
                let new_sig = sdt.0.clone();
                let new_sdt = Sdt {
                    physical_address: sdt.1.physical_address.clone(),
                    length: sdt.1.length.clone(),
                    validated: sdt.1.validated.clone(),
                };
                (new_sig, new_sdt)
            })
            .collect::<Vec<_>>();

        // as before: recreate the Vec of AML tables
        let new_aml_vec = self
            .ssdts
            .iter()
            .map(|table| AmlTable {
                address: table.address,
                length: table.length,
            })
            .collect::<Vec<_>>();

        let new_dsdt = match &self.dsdt {
            Some(dsdt) => Some(AmlTable {
                address: dsdt.address.clone(),
                length: dsdt.length.clone(),
            }),
            None => None,
        };

        Self {
            revision: self.revision.clone(),
            sdts: new_sdt_vec.leak(),
            dsdt: new_dsdt,
            ssdts: new_aml_vec.leak(),
            info: self.info.clone(), // already impled manually
        }
    }
}

// Allow globals
unsafe impl Send for PortableAcpiTables {}
unsafe impl Sync for PortableAcpiTables {}
