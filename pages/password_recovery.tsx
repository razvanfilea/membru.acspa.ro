import {useRouter} from "next/router";
import {useState} from "react";
import {Box, Button, Group, Loader, PasswordInput, Space, Stack, Text, Title} from "@mantine/core";
import {useForm} from "@mantine/form";
import {useSupabaseClient} from "@supabase/auth-helpers-react";

export default function PasswordRecoveryPage() {
    const supabase = useSupabaseClient()
    const [state, setState] = useState<"default" | "resetting" | string>("default");
    const router = useRouter();

    const form = useForm({
        initialValues: {
            password: '',
            confirmPassword: '',
        },

        validate: {
            password: (value) =>
                (value.length >= 8) ? null : "Parola trebuie să aibă cel puțin 8 caractere",
            confirmPassword: (value, values) =>
                value !== values.password ? 'Parolele nu se potrivesc' : null,
        },
        validateInputOnBlur: true
    });

    function FormBottomSection() {
        if (state == 'default')
            return <Button type={'submit'} w={'100%'}>Salvează</Button>

        if (state == 'resetting')
            return <Loader/>

        return <Text color={'red'}>A avut loc o eroare: {state}</Text>
    }

    return (<Box sx={{maxWidth: 480}} mx="auto">
            <Stack>
                <form style={{position: 'relative'}}
                      onSubmit={form.onSubmit(async (values) => {
                          setState('resetting')

                          const {data, error} = await supabase.auth
                              .updateUser({password: values.password});

                          if (data != null)
                              await router.push("/");
                          if (error != null)
                              setState(error.message)
                      })}>

                    <Title>Setare parolă nouă</Title>

                    <Space h={'xl'}/>

                    <PasswordInput
                        label={'Parola'}
                        required={true}
                        placeholder={"Noua parolă"}
                        {...form.getInputProps('password')}
                    />

                    <PasswordInput
                        my={'lg'}
                        required={true}
                        label={'Repetă parola'}
                        placeholder={"Repetă noua parolă"}
                        {...form.getInputProps('confirmPassword')}
                    />

                    <Space h={'xl'}/>

                    <Group position={"center"}>
                        <FormBottomSection/>
                    </Group>
                </form>
            </Stack>

        </Box>
    );
}