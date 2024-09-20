#[macro_export]
macro_rules! setup_constructor {
    (
        struct_name: [$S:ident],
        java_class_generics: [$($G:ident,)*],
        input_names: [$($input_name:ident,)*],
        input_traits: [$($input_trait:path,)*],
        jvm_op_traits: [$($jvm_op_trait:path,)*],
        output_trait: [$output_trait:path],
        prepare_inputs: [$($prepare_inputs:tt)*],
        descriptor: [$descriptor:expr],
        jni_descriptor: [$jni_descriptor:expr],
    ) => {
        pub fn new(
            $($input_name : impl $input_trait,)*
        ) -> impl $output_trait {
            struct Impl<
                $($G,)*
                $($input_name,)*
            > {
                $($input_name : $input_name,)*
                phantom: ::core::marker::PhantomData<($($G,)*)>,
            }

            impl<$($G,)* $($input_name,)*> ::core::clone::Clone for Impl<$($G,)* $($input_name,)*>
            where
                $($G: duchess::JavaObject,)*
                $($input_name: $jvm_op_trait,)*
            {
                fn clone(&self) -> Self {
                    Impl {
                        $($input_name : ::core::clone::Clone(&self.$input_name),)*
                        phantom: ::core::marker::PhantomData,
                    }
                }
            }

            impl<$($G,)* $($input_name,)*> duchess::prelude::JvmOp for Impl<$($G,)* $($input_name,)*>
            where
                $($G: duchess::JavaObject,)*
                $($input_name: $jvm_op_trait,)*
            {
                type Output<'jvm> = duchess::Local<'jvm, $S<$($G,)*>>;

                fn do_jni<'jvm>(
                    self,
                    jvm: &mut duchess::Jvm<'jvm>,
                ) -> duchess::LocalResult<'jvm, Self::Output<'jvm>> {
                    use duchess::plumbing::once_cell::sync::OnceCell;

                    $($prepare_inputs)*

                    let class = <$S<$($G,)*> as duchess::JavaObject>::class(jvm)?;

                    // Cache the method id for the constructor -- note that we only have one cache
                    // no matter how many generic monomorphizations there are. This makes sense
                    // given Java's erased-based generics system.
                    static CONSTRUCTOR: OnceCell<duchess::plumbing::MethodPtr> = OnceCell::new();
                    let constructor = CONSTRUCTOR.get_or_try_init(|| {
                        duchess::plumbing::find_constructor(jvm, &class, $jni_descriptor)
                    })?;

                    let env = jvm.env();
                    let obj: ::core::option::Option<duchess::Local<$S<$($G,)*>>> = unsafe {
                        env.invoke(|env| env.NewObjectA, |env, f| f(
                            env,
                            duchess::plumbing::JavaObjectExt::as_raw(&*class).as_ptr(),
                            constructor.as_ptr(),
                            [
                                $(duchess::plumbing::IntoJniValue::into_jni_value($input_names),)*
                            ].as_ptr(),
                        ))
                    }?;
                    obj.ok_or_else(|| {
                        // NewObjectA should only return a null pointer when an exception occurred in the
                        // constructor, so reaching here is a strange JVM state
                        duchess::Error::JvmInternal(format!(
                            "failed to create new `{}` via constructor `{}`",
                            stringify!($S), $descriptor,
                        ))
                    })
                }
            }


            impl<$($G,)* $($input_name,)*> ::core::ops::Deref for Impl<$($G,)* $($input_name,)*>
            where
                $($G: duchess::JavaObject,)*
                $($input_name: $jvm_op_trait,)*
            {
                type Target = <$S<$($G,)*> as duchess::plumbing::JavaView>::OfOp<Self>;

                fn deref(&self) -> &Self::Target {
                    <Self::Target as duchess::plumbing::FromRef<_>>::from_ref(self)
                }
            }

            Impl {
                $($input_names: $input_names.into_op(),)*
                phantom: ::core::default::Default::default()
            }
        }
    }
}
