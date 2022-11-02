import {Box, Button, Group, Paper, Space, Stack, Text, TextInput, Title} from "@mantine/core";
import {MdAlternateEmail} from "react-icons/md";
import {useForm} from "@mantine/form";
import {useEffect, useState} from "react";
import {useRouter} from "next/router";
import {useProfile} from "../components/ProfileProvider";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";

const REGEX_EMAIL = /^(([^<>()[\]\\.,;:\s@"]+(\.[^<>()[\]\\.,;:\s@"]+)*)|(".+"))@((\[\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}])|(([a-zA-Z\-\d]+\.)+[a-zA-Z]{2,}))$/

const enum Status {
    None,
    Failed,
    Loading,
    Success
}

export default function LoginForm() {
    const supabase = useSupabaseClient<Database>()
    const router = useRouter()
    const profileData = useProfile()

    const form = useForm({
        initialValues: {
            email: '',
        },

        validate: {
            email: (value) => REGEX_EMAIL.test(value.toLowerCase()) ? null : "Email invalid",
        },
        validateInputOnBlur: true
    });

    const [resetStatus, setResetStatus] = useState(Status.None)

    useEffect(() => {
        if (!profileData.isLoading && profileData.profile != null) {
            router.back()
        }
    }, [profileData, router])

    return <Box sx={{maxWidth: 480}} mx="auto">
        <Stack>
            <form style={{position: 'relative'}} onSubmit={
                form.onSubmit(async (values) => {
                    setResetStatus(Status.Loading)
                    const {error} = await supabase.auth.resetPasswordForEmail(values.email)

                    setResetStatus(error == null ? Status.Success : Status.Failed)
                })}>

                <Title>Resetare parolă</Title>

                <Space h={"lg"}/>

                <TextInput
                    {...form.getInputProps('email')}
                    type={"email"}
                    label={"Email:"}
                    placeholder={"mail@example.com"}
                    required={true}
                    icon={<MdAlternateEmail size={14}/>}
                />

                <Space h="lg"/>

                <Group position="right" mt="md">
                    <Button type={"submit"} disabled={resetStatus == Status.Success}
                            loading={resetStatus == Status.Loading}>Resetează parola</Button>
                </Group>
            </form>

            {resetStatus == Status.Failed &&
                <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                    backgroundColor: theme.colors.orange,
                })}>
                    <Text>A fost întâmpinată o eroare!</Text>
                </Paper>
            }

            {resetStatus == Status.Success &&
                <Text>Un email a fost trimis la adresa ta, de unde îți vei putea reseta parola</Text>
            }
        </Stack>

    </Box>
}
