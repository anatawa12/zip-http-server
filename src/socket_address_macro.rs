macro_rules! tcp_unix_impl {
    (
        $(#[$enum_meta: meta])*
        pub(crate) enum $enum_name: ident {
            TCP($enum_v_tcp: ty),
            UnixDomainSocket($enum_v_udp: ty),
        }
        $(
            $(#[$impl_meta: meta])*
            impl $impl_trait: ident {
                $( type $alias_n: ident = $alias_v: ty; )*
                $(
                    $(#[$fn_meta: meta])*
                    fn $fn_name:ident (
                        self: Pin<&mut Self>,
                        $( $param_n: ident : $param_t: ty ),* $(,)?
                    ) -> ( $( $fn_result: tt )* ) {
                        $($fn_body: tt)*
                    }
                )*
            }
        )*
    ) => {
        $(#[$enum_meta])*
        pub(crate) enum $enum_name {
            TCP($enum_v_tcp),
            #[cfg(unix)]
            UnixDomainSocket($enum_v_udp),
        }
        $(
            $(#[$impl_meta])*
            impl $impl_trait for $enum_name {
                $( type $alias_n = $alias_v; )*
                $(
                    $(#[$fn_meta])*
                    fn $fn_name (self: Pin<&mut Self>, $( $param_n : $param_t ),* ) -> $( $fn_result )* {
                        tcp_unix_impl!(
                            @fn_body
                            ( $fn_name )
                            ( self )
                            ( $( $fn_result )* )
                            ( $( $param_n ),* )
                            ( $( $fn_body )* )
                        )
                    }
                )*
            }
        )*
    };
    (
        @fn_body
        ( $fn_name: ident )
        ( $self_param: tt )
        ( $( $fn_result: tt )* )
        ( $( $parm_name: ident),* )
        ( simple_return )
    ) => {
        match Pin::into_inner($self_param) {
            Self::TCP(ref mut main) => Pin::new(main)
                .$fn_name($($parm_name),*),
            #[cfg(unix)]
            Self::UnixDomainSocket(ref mut main) => Pin::new(main)
                .$fn_name($($parm_name),*),
        }
    };
    (
        @fn_body
        ( $fn_name: ident )
        ( $self_param: tt )
        ( Poll<Option<Result<$fn_result_body: ty, $fn_result_err: ty>>> )
        ( $( $parm_name: ident ),* )
        ( map_result )
    ) => {
        match Pin::into_inner($self_param) {
            Self::TCP(ref mut main) => Pin::new(main)
                .$fn_name($($parm_name),*)
                .map(|o| o.map(|r| r.map(<$fn_result_body>::TCP))),
            #[cfg(unix)]
            Self::UnixDomainSocket(ref mut main) => Pin::new(main)
                .$fn_name($($parm_name),*)
                .map(|o| o.map(|r| r.map(<$fn_result_body>::UnixDomainSocket))),
        }
    };
}
