import {Box, Button, Group, Paper, PasswordInput, Space, Stack, Text, TextInput, Title} from "@mantine/core";
import {MdAlternateEmail, MdPerson} from "react-icons/md";
import {useForm} from "@mantine/form";
import {useEffect, useState} from "react";
import {useRouter} from "next/router";
import {useAuth} from "../components/AuthProvider";

const enum RegisterState {
    None,
    Failed,
    Loading,
    Success,
}

export default function LoginForm() {
    const auth = useAuth()
    const router = useRouter()
    const form = useForm({
        initialValues: {
            name: '',
            password: '',
            confirmPassword: '',
        },

        validate: {
            name: (value) => value.length <= 64 ? null : "Numele nu poate fi mai lung de 64 de litere",
            password: (value) =>
                (value.length >= 8) ? null : "Parola trebuie să aibă cel puțin 8 caractere",
            confirmPassword: (value, values) =>
                value !== values.password ? 'Parolele nu se potrivesc' : null,
        },
        validateInputOnBlur: true
    });

    const [registerState, setRegisterState] = useState(RegisterState.None)

    useEffect(() => {
        if (auth.profile != null || registerState == RegisterState.Success) {
            router.push('/')
        }
    }, [auth.profile, registerState, router])

    return (<>
        <Box sx={{maxWidth: 480}} mx="auto">
            <Stack>
                <form style={{position: 'relative'}} onSubmit={
                    form.onSubmit(async (values) => {
                        setRegisterState(RegisterState.Loading)
                        const success = await auth.changePassword(values.name, values.password)

                        setRegisterState(success ? RegisterState.Success : RegisterState.Failed)
                    })}>

                    <Title>Înregistrare cont</Title>

                    <Space h={"lg"}/>

                    <TextInput
                        type={"email"}
                        label={"Email:"}
                        contentEditable={'false'}
                        value={auth.user?.email}
                        icon={<MdAlternateEmail size={14}/>}
                    />

                    <Space h="md"/>

                    <TextInput
                        {...form.getInputProps('name')}
                        type={"text"}
                        label={"Nume:"}
                        placeholder={"Nume"}
                        required={true}
                        icon={<MdPerson size={14}/>}
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
                        <Button type={"submit"}
                                loading={registerState == RegisterState.Loading}>Înregistrare</Button>
                    </Group>
                </form>

                {registerState == RegisterState.Failed &&
                    <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                        backgroundColor: theme.colors.orange,
                    })}>
                        <Text>A fost întâmpinată o eroare la înregistrare!</Text>
                    </Paper>
                }

                {registerState == RegisterState.Success &&
                    <Text>Te-ai înregistrat cu succes!</Text>
                }
            </Stack>

        </Box>
    </>)
}
