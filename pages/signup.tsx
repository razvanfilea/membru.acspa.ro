import {Box, Button, Group, Paper, PasswordInput, Space, Stack, Text, TextInput, Title} from "@mantine/core";
import {MdAccountBox, MdAlternateEmail, MdPerson} from "react-icons/md";
import {useForm} from "@mantine/form";
import {useEffect, useState} from "react";
import {useRouter} from "next/router";
import Link from "next/link";
import {useAuth} from "../components/AuthProvider";

const REGEX_EMAIL = /^(([^<>()[\]\\.,;:\s@"]+(\.[^<>()[\]\\.,;:\s@"]+)*)|(".+"))@((\[\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}])|(([a-zA-Z\-\d]+\.)+[a-zA-Z]{2,}))$/

const enum RegisterState {
    None,
    Failed,
    Loading,
    Success,
    LoginSuccess
}

export default function LoginForm() {
    const auth = useAuth()
    const router = useRouter()
    const form = useForm({
        initialValues: {
            name: '',
            email: '',
            password: '',
            confirmPassword: '',
        },

        validate: {
            name: (value) => value.length <= 64 ? null : "Numele nu poate fi mai lung de 64 de litere",
            email: (value) => REGEX_EMAIL.test(value.toLowerCase()) ? null : "Email invalid",
            password: (value) =>
                (value.length >= 8) ? null : "Parola trebuie să aibă cel puțin 8 caractere",
            confirmPassword: (value, values) =>
                value !== values.password ? 'Parolele nu se portivesc' : null,
        },
        validateInputOnChange: true
    });

    const [registerState, setRegisterState] = useState(RegisterState.None)

    useEffect(() => {
        if (auth.user != null) {
            router.push('/')
        }
    }, [auth.user, router])

    return (<>
        <Box sx={{maxWidth: 480}} mx="auto">
            <Stack>
                <form style={{position: 'relative'}} onSubmit={
                    form.onSubmit(async (values) => {
                        setRegisterState(RegisterState.Loading)
                        const success = await auth.signUp({email: values.email, password: values.password}, values.name)

                        setRegisterState(success ? RegisterState.LoginSuccess : RegisterState.Failed)
                    })}>

                    <Title>Înregistrare cont</Title>

                    <Space h={"lg"}/>

                    <TextInput
                        {...form.getInputProps('name')}
                        type={"text"}
                        label={"Nume:"}
                        placeholder={"Nume"}
                        required={true}
                        icon={<MdPerson size={14}/>}
                    />

                    <Space h="md"/>

                    <TextInput
                        {...form.getInputProps('email')}
                        type={"email"}
                        label={"Email:"}
                        placeholder={"mail@example.com"}
                        required={true}
                        icon={<MdAlternateEmail size={14}/>}
                    />

                    <Space h="md"/>

                    <PasswordInput
                        {...form.getInputProps('password')}
                        label={"Parola:"}
                        placeholder={"Parola"}
                        required={true}
                    />

                    <Space h="md"/>

                    <PasswordInput
                        {...form.getInputProps('confirmPassword')}
                        label={"Confirmă parola:"}
                        placeholder={"Confirmă parola"}
                        required={true}
                    />

                    <Space h="lg"/>

                    <Group position="apart" mt="md">
                        <Link href={"/login"} passHref={true}>
                            <Button variant={"outline"} leftIcon={<MdAccountBox size={14}/>}>Am deja cont!</Button>
                        </Link>

                        <Button type={"submit"}
                                loading={registerState == RegisterState.Loading}>înregistrare</Button>
                    </Group>
                </form>

                {registerState == RegisterState.Failed &&
                    <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                        backgroundColor: theme.colors.orange,
                    })}>
                        <Text>A fost întâmpinată o eroare!</Text>
                    </Paper>
                }

                {registerState == RegisterState.Success &&
                    <Text>Te-ai înregistrat cu succes!</Text>
                }
            </Stack>

        </Box>
    </>)
}
