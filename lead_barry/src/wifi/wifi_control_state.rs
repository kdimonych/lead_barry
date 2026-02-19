pub enum WiFiControlerState<IdleState, JoinedState, ApState> {
    Uninitialized,
    Idle(IdleState),
    Joined(JoinedState),
    Ap(ApState),
}

#[allow(dead_code)]
impl<IdleState, JoinedState, ApState> WiFiControlerState<IdleState, JoinedState, ApState> {
    pub fn is_idle(&self) -> bool {
        matches!(self, WiFiControlerState::Idle(_))
    }

    pub fn is_joined(&self) -> bool {
        matches!(self, WiFiControlerState::Joined(_))
    }

    pub fn is_ap(&self) -> bool {
        matches!(self, WiFiControlerState::Ap(_))
    }
    pub fn is_uninitialized(&self) -> bool {
        matches!(self, WiFiControlerState::Uninitialized)
    }

    pub fn change<Modifier>(&mut self, modifier: Modifier)
    where
        Modifier: FnOnce(
            WiFiControlerState<IdleState, JoinedState, ApState>,
        ) -> WiFiControlerState<IdleState, JoinedState, ApState>,
    {
        let old_state = core::mem::replace(self, WiFiControlerState::Uninitialized);
        let new_state = modifier(old_state);
        *self = new_state;
    }

    pub async fn change_async<Modifier>(&mut self, modifier: Modifier)
    where
        Modifier: AsyncFnOnce(
            WiFiControlerState<IdleState, JoinedState, ApState>,
        ) -> WiFiControlerState<IdleState, JoinedState, ApState>,
    {
        let old_state = core::mem::replace(self, WiFiControlerState::Uninitialized);
        let new_state = modifier(old_state).await;
        *self = new_state;
    }

    pub fn as_ref(&self) -> WiFiControlerState<&IdleState, &JoinedState, &ApState> {
        match self {
            WiFiControlerState::Uninitialized => WiFiControlerState::Uninitialized,
            WiFiControlerState::Idle(ctrl) => WiFiControlerState::Idle(ctrl),
            WiFiControlerState::Joined(ctrl) => WiFiControlerState::Joined(ctrl),
            WiFiControlerState::Ap(ctrl) => WiFiControlerState::Ap(ctrl),
        }
    }

    pub fn as_mut(&mut self) -> WiFiControlerState<&mut IdleState, &mut JoinedState, &mut ApState> {
        match self {
            WiFiControlerState::Uninitialized => WiFiControlerState::Uninitialized,
            WiFiControlerState::Idle(ctrl) => WiFiControlerState::Idle(ctrl),
            WiFiControlerState::Joined(ctrl) => WiFiControlerState::Joined(ctrl),
            WiFiControlerState::Ap(ctrl) => WiFiControlerState::Ap(ctrl),
        }
    }

    pub fn map<FIdle, FJoined, FAp, NewIdleState, NewJoinedState, NewApState>(
        self,
        f_idle: FIdle,
        f_joined: FJoined,
        f_ap: FAp,
    ) -> WiFiControlerState<NewIdleState, NewJoinedState, NewApState>
    where
        FIdle: FnOnce(IdleState) -> NewIdleState,
        FJoined: FnOnce(JoinedState) -> NewJoinedState,
        FAp: FnOnce(ApState) -> NewApState,
    {
        match self {
            WiFiControlerState::Uninitialized => WiFiControlerState::Uninitialized,
            WiFiControlerState::Idle(state) => WiFiControlerState::Idle(f_idle(state)),
            WiFiControlerState::Joined(state) => WiFiControlerState::Joined(f_joined(state)),
            WiFiControlerState::Ap(state) => WiFiControlerState::Ap(f_ap(state)),
        }
    }

    pub fn map_idle<FIdle, NewIdleState>(
        self,
        f_idle: FIdle,
    ) -> WiFiControlerState<NewIdleState, JoinedState, ApState>
    where
        FIdle: FnOnce(IdleState) -> NewIdleState,
    {
        match self {
            WiFiControlerState::Uninitialized => WiFiControlerState::Uninitialized,
            WiFiControlerState::Idle(state) => WiFiControlerState::Idle(f_idle(state)),
            WiFiControlerState::Joined(state) => WiFiControlerState::Joined(state),
            WiFiControlerState::Ap(state) => WiFiControlerState::Ap(state),
        }
    }

    pub fn map_joined<FJoined, NewJoinedState>(
        self,
        f_joined: FJoined,
    ) -> WiFiControlerState<IdleState, NewJoinedState, ApState>
    where
        FJoined: FnOnce(JoinedState) -> NewJoinedState,
    {
        match self {
            WiFiControlerState::Uninitialized => WiFiControlerState::Uninitialized,
            WiFiControlerState::Idle(state) => WiFiControlerState::Idle(state),
            WiFiControlerState::Joined(state) => WiFiControlerState::Joined(f_joined(state)),
            WiFiControlerState::Ap(state) => WiFiControlerState::Ap(state),
        }
    }

    pub fn map_ap<FAp, NewApState>(
        self,
        f_ap: FAp,
    ) -> WiFiControlerState<IdleState, JoinedState, NewApState>
    where
        FAp: FnOnce(ApState) -> NewApState,
    {
        match self {
            WiFiControlerState::Uninitialized => WiFiControlerState::Uninitialized,
            WiFiControlerState::Idle(state) => WiFiControlerState::Idle(state),
            WiFiControlerState::Joined(state) => WiFiControlerState::Joined(state),
            WiFiControlerState::Ap(state) => WiFiControlerState::Ap(f_ap(state)),
        }
    }
}
