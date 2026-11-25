use crate::{
    ir::{DomainId, Error},
    module::DomainLabel,
    signal::{Signal, SignalId, SignalView},
    sim::Simulator,
    value::Shape,
};

impl Simulator {
    fn check_signal(&self, id: SignalId) -> Result<(), Error> {
        if id.owner != self.design().id {
            return Err(Error("signal belongs to a different design".into()));
        }
        Ok(())
    }

    pub fn get<T: Shape>(&self, signal: impl Into<Signal<T>>) -> Result<T, Error> {
        let id = signal.into().id;
        self.check_signal(id)?;
        Ok(T::from_bits(self.get_signal(id.raw)?))
    }

    pub fn set<T: Shape>(
        &mut self,
        signal: impl Into<Signal<T>>,
        value: impl Into<T>,
    ) -> Result<(), Error> {
        let id = signal.into().id;
        self.check_signal(id)?;
        self.set_signal(id.raw, value.into().to_bits())
    }

    pub fn get_view<T: Shape>(&self, view: SignalView<T>) -> Result<T, Error> {
        self.check_signal(view.id)?;
        let bits = self.get_signal(view.id.raw)?;
        if view.offset > bits.width() || T::WIDTH > bits.width() - view.offset {
            return Err(Error("signal view is out of bounds".into()));
        }
        Ok(T::from_bits(&bits.slice(view.offset, T::WIDTH)))
    }

    pub fn set_view<T: Shape>(
        &mut self,
        view: SignalView<T>,
        value: impl Into<T>,
    ) -> Result<(), Error> {
        self.check_signal(view.id)?;
        let mut bits = self.get_signal(view.id.raw)?.clone();
        if view.offset > bits.width() || T::WIDTH > bits.width() - view.offset {
            return Err(Error("signal view is out of bounds".into()));
        }
        bits.replace(view.offset, &value.into().to_bits());
        self.set_signal(view.id.raw, bits)
    }

    pub fn tick_domain<L: DomainLabel>(&mut self) -> Result<(), Error> {
        self.tick(DomainId::of::<L>())
    }
}
