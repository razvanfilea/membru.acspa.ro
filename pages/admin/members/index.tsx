import 'dayjs/locale/ro';
import {Button, Card, Loader, Modal, Stack, TextInput} from "@mantine/core";
import React, {useState} from "react";
import {useForm} from "@mantine/form";
import {dateToISOString} from "../../../utils/date";
import {AdminTopBar} from "../../../components/AdminInput";
import {Database} from "../../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {useExitIfNotFounder} from "../../../utils/admin_tools";
import {UserProfileLayout} from "../../../components/UserProfileLayout";
import {useRouter} from "next/router";
import useProfilesQuery from "../../../hooks/useProfilesQuery";

export default function MembersList() {
    const supabase = useSupabaseClient<Database>()
    const router = useRouter()

    const {data: allProfiles, isLoading} = useProfilesQuery()
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const newInviteForm = useForm({
        initialValues: {
            date: new Date(),
            startHour: 0,
            guestName: '',
            guestType: 'antrenament',
        },
        validate: {
            guestName: (value) => (value.length >= 3) ? null : "Numele invitatului este prea scurt",
            startHour: (value) => value !== 0 ? null : "Ora de început trebuie să fie diferită de 0",
        },
        validateInputOnBlur: true
    });

    useExitIfNotFounder();

    return (<>
        <Modal
            opened={createModalOpened}
            onClose={() => setCreateModalOpened(false)}
            title="Adaugă o invitație"
        >
            <form style={{position: 'relative'}} onSubmit={
                newInviteForm.onSubmit(async (values) => {
                    setCreateModalOpened(false)
                    console.log(values.date)
                    const newGuest = {
                        start_date: dateToISOString(values.date),
                        start_hour: values.startHour,
                        guest_name: values.guestName,
                        special: values.guestType === 'special',
                    }
                    newInviteForm.reset()

                    const {error} = await supabase.from('guest_invites').insert([newGuest])
                    console.log(error)
                })}>

                <Stack>

                    <TextInput
                        {...newInviteForm.getInputProps('guestName')}
                        label={'Nume invitat'}
                        size={'lg'}
                        required={true}/>

                    <Button type={"submit"} color={'green'}>Adaugă</Button>
                </Stack>
            </form>
        </Modal>

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
            <AdminTopBar title={(allProfiles?.length || 0) + ' de membrii'}
                         onAdd={() => router.push('/admin/members/register')}/>

            {isLoading ?
                <Loader/>
                :
                allProfiles!.map((profile) => (
                    <Card key={profile.id} shadow={"xs"}>
                        <UserProfileLayout profile={profile}/>
                    </Card>
                ))
            }

            {}
        </Stack>
    </>)
}
