import {Box, Button, Group, Paper, PasswordInput, Space, Stack, Text, TextInput, Title} from "@mantine/core";
import {MdAlternateEmail, MdPassword} from "react-icons/md";
import {useForm} from "@mantine/form";
import {useEffect, useState} from "react";
import {useRouter} from "next/router";
import Link from "next/link";
import {useSession, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {REGEX_EMAIL_PATTERN} from "../utils/regex";

const enum LoginState {
    None,
    Failed,
    Loading,
    Success
}

export default function LoginForm() {
    const supabase = useSupabaseClient<Database>()
    const session = useSession()
    const router = useRouter()
    const form = useForm({
        initialValues: {
            email: '',
            password: '',
        },

        validate: {
            email: (value) => REGEX_EMAIL_PATTERN.test(value.toLowerCase()) ? null : "Email invalid",
            password: (value) => (value.length >= 8) ? null : "Parola trebuie sa aibă cel putin 8 caractere"
        },
        validateInputOnBlur: true
    });

    const [loginState, setLoginState] = useState(LoginState.None)

    useEffect(() => {
        if (session != null) {
            setLoginState(LoginState.Success)
        }
    }, [session])

    useEffect(() => {
        if (loginState == LoginState.Success) {
            router.back()
        }
    }, [loginState, router])

    return <Box sx={{maxWidth: 480}} mx="auto">
        <Stack>
            <form style={{position: 'relative'}} onSubmit={
                form.onSubmit(async (values) => {
                    setLoginState(LoginState.Loading)
                    const {error} = await supabase.auth.signInWithPassword({email: values.email, password: values.password})

                    setLoginState(error == null ? LoginState.Success : LoginState.Failed)
                })}>

                <Title>Login</Title>

                <Space h={"lg"}/>

                <TextInput
                    {...form.getInputProps('email')}
                    type={"email"}
                    label={"Email:"}
                    placeholder={"mail@example.com"}
                    required={true}
                    icon={<MdAlternateEmail size={14}/>}
                />

                <Space h={"lg"}/>

                <PasswordInput
                    {...form.getInputProps('password')}
                    label={"Parola:"}
                    placeholder={"Parola"}
                    required={true}
                    icon={<MdPassword size={14}/>}
                />

                <Space h="lg"/>

                <Group position="apart" mt="md">
                    <Button variant={'subtle'}><Link href={'/forgot_password'}>Am uitat parola</Link></Button>

                    <Button type={"submit"}
                            loading={loginState == LoginState.Loading}>Logare</Button>
                </Group>
            </form>

            {loginState == LoginState.Failed &&
                <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                    backgroundColor: theme.colors.orange,
                })}>
                    <Text>A fost întâmpinată o eroare!</Text>
                </Paper>
            }

            {loginState == LoginState.Success &&
                <Text>Te-ai logat cu succes!</Text>
            }
        </Stack>

    </Box>
}
