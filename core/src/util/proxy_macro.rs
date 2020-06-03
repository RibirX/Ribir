pub macro proxy_immut_impl(
    $method: ident, {$($path:ident).*},
    $type: ty,
    $call_method: ident,
    ($($arg:ident : $arg_type: ty),*))
{
    #[inline]
    fn $method(&self $(, $arg: $arg_type)*)-> $type {
        self.$($path).* .$call_method($($arg,)*)
    }
}

pub macro proxy_mut_impl(
    $method: ident,
    {$($path:ident).*},
    $type: ty,
    $call_method: ident,
    ($($arg:ident : $arg_type: ty),*))
{
    #[inline]
    fn $method(&mut self $(, $arg: $arg_type)*) -> $type {
        self. $($path).* .$call_method($($arg,)*)
    }
}
