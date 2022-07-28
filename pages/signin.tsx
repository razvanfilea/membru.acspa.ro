import {Box, Button, Group, Paper, PasswordInput, Space, Stack, Text, TextInput, Title} from "@mantine/core";
import {MdAccountBox, MdAlternateEmail, MdPassword} from "react-icons/md";
import {useForm} from "@mantine/form";
import {appwrite, userIsLoggedIn} from "../utils/appwrite_utils";
import {useEffect, useState} from "react";
import {useRouter} from "next/router";
import Link from "next/link";

const REGEX_EMAIL = /^(([^<>()[\]\\.,;:\s@"]+(\.[^<>()[\]\\.,;:\s@"]+)*)|(".+"))@((\[\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}])|(([a-zA-Z\-\d]+\.)+[a-zA-Z]{2,}))$/

enum LoginState {
    None,
    Failed,
    Loading,
    Success
}

export default function LoginForm() {
    const router = useRouter()
    const form = useForm({
        initialValues: {
            email: '',
            password: '',
        },

        validate: {
            email: (value) => REGEX_EMAIL.test(value.toLowerCase()) ? null : "Email invalid",
            password: (value) => (value.length >= 8) ? null : "Parola trebuie sa aiba cel putin 8 caractere"
        },
        validateInputOnChange: true
    });

    const [loginState, setLoginState] = useState(LoginState.None)

    useEffect(() => {
        userIsLoggedIn().then((value) => {
            if (value) setLoginState(LoginState.Success)
        })
    }, [])

    useEffect(() => {
        if (loginState == LoginState.Success) {
            router.back()
        }
    }, [loginState, router])

    return (<>
        <Box sx={{maxWidth: 480}} mx="auto">
            <Stack>
                <form style={{position: 'relative'}} onSubmit={
                    form.onSubmit((values) => {
                        setLoginState(LoginState.Loading)
                        appwrite.account.createEmailSession(values.email, values.password)
                            .then(
                                res => setLoginState(LoginState.Success),
                                err => {
                                    console.log(err)
                                    setLoginState(LoginState.Failed)
                                })
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
                        <Link href={"/signup"} passHref={true}>
                            <Button variant={"outline"} leftIcon={<MdAccountBox size={14}/>}>Nu am cont</Button>
                        </Link>

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
    </>)
}
