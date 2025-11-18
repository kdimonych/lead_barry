pub enum WiFiControlState<IdleState, JoinedState, ApState> {
    Uninitialized,
    Idle(IdleState),
    Joined(JoinedState),
    Ap(ApState),
}
impl<IdleState, JoinedState, ApState> WiFiControlState<IdleState, JoinedState, ApState> {
    pub fn is_idle(&self) -> bool {
        matches!(self, WiFiControlState::Idle(_))
    }

    pub fn is_joined(&self) -> bool {
        matches!(self, WiFiControlState::Joined(_))
    }

    pub fn is_ap(&self) -> bool {
        matches!(self, WiFiControlState::Ap(_))
    }
    pub fn is_uninitialized(&self) -> bool {
        matches!(self, WiFiControlState::Uninitialized)
    }

    pub fn change<Modifier>(&mut self, modifier: Modifier)
    where
        Modifier: FnOnce(
            WiFiControlState<IdleState, JoinedState, ApState>,
        ) -> WiFiControlState<IdleState, JoinedState, ApState>,
    {
        let old_state = core::mem::replace(self, WiFiControlState::Uninitialized);
        let new_state = modifier(old_state);
        *self = new_state;
    }

    pub async fn change_async<Modifier>(&mut self, modifier: Modifier)
    where
        Modifier: AsyncFnOnce(
            WiFiControlState<IdleState, JoinedState, ApState>,
        ) -> WiFiControlState<IdleState, JoinedState, ApState>,
    {
        let old_state = core::mem::replace(self, WiFiControlState::Uninitialized);
        let new_state = modifier(old_state).await;
        *self = new_state;
    }

    pub fn as_ref(&self) -> WiFiControlState<&IdleState, &JoinedState, &ApState> {
        match self {
            WiFiControlState::Uninitialized => WiFiControlState::Uninitialized,
            WiFiControlState::Idle(ctrl) => WiFiControlState::Idle(ctrl),
            WiFiControlState::Joined(ctrl) => WiFiControlState::Joined(ctrl),
            WiFiControlState::Ap(ctrl) => WiFiControlState::Ap(ctrl),
        }
    }

    pub fn as_mut(&mut self) -> WiFiControlState<&mut IdleState, &mut JoinedState, &mut ApState> {
        match self {
            WiFiControlState::Uninitialized => WiFiControlState::Uninitialized,
            WiFiControlState::Idle(ctrl) => WiFiControlState::Idle(ctrl),
            WiFiControlState::Joined(ctrl) => WiFiControlState::Joined(ctrl),
            WiFiControlState::Ap(ctrl) => WiFiControlState::Ap(ctrl),
        }
    }

    pub fn map<FIdle, FJoined, FAp, NewIdleState, NewJoinedState, NewApState>(
        self,
        f_idle: FIdle,
        f_joined: FJoined,
        f_ap: FAp,
    ) -> WiFiControlState<NewIdleState, NewJoinedState, NewApState>
    where
        FIdle: FnOnce(IdleState) -> NewIdleState,
        FJoined: FnOnce(JoinedState) -> NewJoinedState,
        FAp: FnOnce(ApState) -> NewApState,
    {
        match self {
            WiFiControlState::Uninitialized => WiFiControlState::Uninitialized,
            WiFiControlState::Idle(state) => WiFiControlState::Idle(f_idle(state)),
            WiFiControlState::Joined(state) => WiFiControlState::Joined(f_joined(state)),
            WiFiControlState::Ap(state) => WiFiControlState::Ap(f_ap(state)),
        }
    }

    pub fn map_idle<FIdle, NewIdleState>(
        self,
        f_idle: FIdle,
    ) -> WiFiControlState<NewIdleState, JoinedState, ApState>
    where
        FIdle: FnOnce(IdleState) -> NewIdleState,
    {
        match self {
            WiFiControlState::Uninitialized => WiFiControlState::Uninitialized,
            WiFiControlState::Idle(state) => WiFiControlState::Idle(f_idle(state)),
            WiFiControlState::Joined(state) => WiFiControlState::Joined(state),
            WiFiControlState::Ap(state) => WiFiControlState::Ap(state),
        }
    }

    pub fn map_joined<FJoined, NewJoinedState>(
        self,
        f_joined: FJoined,
    ) -> WiFiControlState<IdleState, NewJoinedState, ApState>
    where
        FJoined: FnOnce(JoinedState) -> NewJoinedState,
    {
        match self {
            WiFiControlState::Uninitialized => WiFiControlState::Uninitialized,
            WiFiControlState::Idle(state) => WiFiControlState::Idle(state),
            WiFiControlState::Joined(state) => WiFiControlState::Joined(f_joined(state)),
            WiFiControlState::Ap(state) => WiFiControlState::Ap(state),
        }
    }

    pub fn map_ap<FAp, NewApState>(
        self,
        f_ap: FAp,
    ) -> WiFiControlState<IdleState, JoinedState, NewApState>
    where
        FAp: FnOnce(ApState) -> NewApState,
    {
        match self {
            WiFiControlState::Uninitialized => WiFiControlState::Uninitialized,
            WiFiControlState::Idle(state) => WiFiControlState::Idle(state),
            WiFiControlState::Joined(state) => WiFiControlState::Joined(state),
            WiFiControlState::Ap(state) => WiFiControlState::Ap(f_ap(state)),
        }
    }
}
