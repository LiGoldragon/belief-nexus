#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    clippy::type_complexity
)]
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub struct Call_Data {
    pub caller: shared_translator::Caller,
    pub query: meta_signal_belief_nexus::Input,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Input {
    Call(Call_Data),
    RestoreTranslatorConfiguration,
}
#[rustfmt::skip]
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "datom", derive(datom_codec::Datomizable, datom_codec::Composing))]
pub enum Output {
    Answered(meta_signal_belief_nexus::Output),
    Restored(meta_signal_belief_nexus::TranslatorConfiguration),
    Failed(String),
}
#[rustfmt::skip]
#[allow(clippy::result_large_err, clippy::ptr_arg, clippy::borrowed_box)]
pub trait Answering {
    fn call(
        &self,
        call: &Call_Data,
    ) -> std::result::Result<meta_signal_belief_nexus::Output, Output>;
    fn restore_translator_configuration(
        &self,
    ) -> std::result::Result<meta_signal_belief_nexus::TranslatorConfiguration, Output>;
}
#[rustfmt::skip]
pub trait Answerable {
    type Answer;
    fn answer<Nexus: Answering>(&self, nexus: &Nexus) -> Self::Answer;
}
#[rustfmt::skip]
impl Answerable for Input {
    type Answer = Output;
    fn answer<Nexus: Answering>(&self, nexus: &Nexus) -> Output {
        match self {
            Input::Call(carried) => {
                nexus.call(carried).map_or_else(|refusal| refusal, Output::Answered)
            }
            Input::RestoreTranslatorConfiguration => {
                nexus
                    .restore_translator_configuration()
                    .map_or_else(|refusal| refusal, Output::Restored)
            }
        }
    }
}
