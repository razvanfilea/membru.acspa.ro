import 'dayjs/locale/ro';
import {Button, Paper, Select, Stack, Text, TextInput, Title} from "@mantine/core";
import React, {useEffect, useState} from "react";
import {useForm} from "@mantine/form";
import {Database} from "../../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {useExitIfNotFounder} from "../../../utils/admin_tools";
import {MdAlternateEmail, MdGroups, MdPassword, MdPerson} from "react-icons/md";
import {REGEX_EMAIL_PATTERN} from "../../../utils/regex";
import {createClient} from "@supabase/supabase-js";

const enum RegisterState {
    None,
    Failed,
    Loading,
    Success,
}

export default function CreateMember() {
    const supabase = useSupabaseClient<Database>()
    const [serviceRole, setServiceRole] = useState<string | null>(null)
    const [registerState, setRegisterState] = useState(RegisterState.None)
    const [error, setError] = useState<string | null>(null)
    const [memberRoles, setMemberRoles] = useState<string[]>(['Membru'])

    const form = useForm({
        initialValues: {
            email: '',
            name: '',
            role: 'Membru',
            password: '',
        },

        validate: {
            email: (value) => REGEX_EMAIL_PATTERN.test(value.toLowerCase()) ? null : "Email invalid",
            name: (value) => (value.length <= 64) ? (value.length >= 3 ? null : "Numele este prea scurt") : "Numele nu poate fi mai lung de 64 de litere",
            role: (value) => memberRoles.includes(value),
            password: (value) =>
                (value.length >= 8) ? null : "Parola trebuie să aibă cel puțin 8 caractere",
        },
        validateInputOnBlur: true
    });

    useExitIfNotFounder();

    useEffect(() => {
        supabase.from('admin_vars')
            .select('*')
            .then(value => {
                if (value.data != null) {
                    setServiceRole(value.data[0].service_role)
                }
            })

        supabase.from('member_roles')
            .select('role')
            .then(value =>
                setMemberRoles(value.data?.map(it => it.role) ?? []))

        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    return (<>
        <Stack sx={(theme) => ({
            padding: theme.spacing.lg,
            '@media (max-width: 900px)': {
                paddingLeft: theme.spacing.md,
                paddingRight: theme.spacing.md,
            },
            '@media (max-width: 600px)': {
                paddingLeft: 0,
                paddingRight: 0,
            }
        })}>
            <form style={{position: 'relative'}} onSubmit={
                form.onSubmit(async (values) => {
                    const supabaseAdmin = createClient(process.env.NEXT_PUBLIC_SUPABASE_URL!, serviceRole!, {
                        auth: {
                            autoRefreshToken: false,
                            persistSession: false
                        }
                    })
                    const adminAuthClient = supabaseAdmin.auth.admin

                    setRegisterState(RegisterState.Loading)

                    const {data, error} = await adminAuthClient.createUser({
                        email: values.email,
                        password: values.password,
                        email_confirm: true,
                    })

                    if (error != null) {
                        setError(`Nume eroare: ${error.name}. Mesaj eroare: ${error.message}`)
                        setRegisterState(RegisterState.Failed)
                        return
                    }

                    const result = await supabase
                        .from('profiles')
                        .insert([{
                            id: data.user?.id!,
                            name: values.name
                        }])

                    if (result.error != null) {
                        setError(`Eroare: ${result.error.message}`)
                        setRegisterState(RegisterState.Failed)
                        return
                    }

                    setRegisterState(RegisterState.Success)
                })}>

                <Title>Înregistrare utilizator nou</Title>

                <TextInput
                    {...form.getInputProps('email')}
                    type={"email"}
                    label={"Email:"}
                    placeholder={"Email"}
                    required={true}
                    icon={<MdAlternateEmail size={14}/>}
                    pt={'lg'}
                    pb={'md'}
                />

                <TextInput
                    {...form.getInputProps('name')}
                    type={"text"}
                    label={"Nume:"}
                    placeholder={"Nume"}
                    required={true}
                    icon={<MdPerson size={14}/>}
                    pb={'md'}
                />

                <Select
                    {...form.getInputProps('role')}
                    type={"text"}
                    label={"Rol:"}
                    placeholder={"Role"}
                    required={true}
                    icon={<MdGroups size={14}/>}
                    data={memberRoles}
                    pb={'md'}
                />

                <TextInput
                    {...form.getInputProps('password')}
                    label={"Parola:"}
                    type={"text"}
                    placeholder={"Parola"}
                    required={true}
                    icon={<MdPassword size={14}/>}
                    pb={'lg'}
                />

                <Button type={"submit"}
                        loading={registerState == RegisterState.Loading}>Înregistrează</Button>
            </form>

            {registerState == RegisterState.Failed &&
                <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                    backgroundColor: theme.colors.orange,
                })}>
                    <Text>A fost întâmpinată o eroare la înregistrare:</Text>
                    <Text>{error}</Text>
                </Paper>
            }

            {registerState == RegisterState.Success &&
                <Text>Utilizatorul a fost înregistrat cu succes!</Text>
            }
        </Stack>
    </>)
}
