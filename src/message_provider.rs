use crate::lang::LangMessage;

pub trait MessageProvider: Sync + Send {
    fn set_message(&self, message: LangMessage);
    fn get_message(&self) -> Option<LangMessage>;
    fn clear(&self);
}
