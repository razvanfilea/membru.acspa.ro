import 'dayjs/locale/ro';
import {Button, Card, Center, Loader, Modal, NumberInputHandlers, Radio, Stack, TextInput} from "@mantine/core";
import React, {useEffect, useMemo, useRef, useState} from "react";
import {useProfile} from "../../components/ProfileProvider";
import {GuestInvite, Location, LocationName, Profile} from "../../types/wrapper";
import {useForm} from "@mantine/form";
import {DatePickerInput} from "@mantine/dates";
import {dateToISOString, isDateWeekend} from "../../utils/date";
import {useListState} from "@mantine/hooks";
import GuestInviteComponent from "../../components/GuestInvite";
import {AdminHourInput, AdminTopBar} from "../../components/AdminInput";
import {createPagesBrowserClient} from "@supabase/auth-helpers-nextjs";
import {Database} from "../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {useExitIfNotFounder} from "../../utils/admin_tools";

interface IParams {
    location: Location
}

export default function GuestManager(params: IParams) {
    const supabase = useSupabaseClient<Database>()
    const game_location = params.location
    const profileData = useProfile()

    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const [guests, guestHandler] = useListState<GuestInvite>([])
    const [isLoading, setIsLoading] = useState(true)
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const hourInputHandlers = useRef<NumberInputHandlers>();
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

    useEffect(() => {
        supabase.from('profiles').select('*').then(value => {
            if (value.data != null) {
                setAllProfiles(value.data)
            }
        })

        fetchGuests().then(data => guestHandler.setState(data))
        setIsLoading(false)
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    async function fetchGuests() {
        const {data} = await supabase.from('guest_invites')
            .select('*')
            .order('start_date', {ascending: false})
            .order('start_hour', {ascending: true})
            .limit(50)

        return data || []
    }

    const hasSelectedWeekend = useMemo(() => isDateWeekend(newInviteForm.values.date),
        [newInviteForm.values.date])

    if (profileData.isLoading || isLoading)
        return <Center> <Loader/> </Center>;

    if (profileData.profile == null)
        return (<></>)

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
                    guestHandler.setState(await fetchGuests())
                })}>

                <Stack>

                    <TextInput
                        {...newInviteForm.getInputProps('guestName')}
                        label={'Nume invitat'}
                        size={'lg'}
                        required={true}/>

                    <Radio.Group
                        label={"Tip de invitat"}
                        withAsterisk
                        size={'lg'}
                        {...newInviteForm.getInputProps('guestType')}>
                        <Stack py={'sm'}>
                            <Radio value={'special'} label={'Invitat Special'}/>
                            <Radio value={'antrenament'} label={'Invitat Antrenament'}/>
                        </Stack>
                    </Radio.Group>

                    <DatePickerInput
                        {...newInviteForm.getInputProps('date')}
                        placeholder="Alege data"
                        label="Data"
                        withAsterisk locale="ro"
                        minDate={new Date()}
                        clearable={false}
                        size={'lg'}
                        dropdownType={'modal'}
                        valueFormat="YYYY-MM-DD"/>

                    <AdminHourInput
                        formProps={newInviteForm.getInputProps('startHour')}
                        inputHandler={hourInputHandlers}
                        gameLocation={game_location}
                        isWeekend={hasSelectedWeekend}/>

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
            <AdminTopBar
                title={'Invitați:'}
                onAdd={() => setCreateModalOpened(true)}/>

            {guests.map((guest) => (
                <Card key={guest.start_date + guest.start_hour + guest.guest_name} shadow={"xs"}>
                    {GuestInviteComponent(
                        guest,
                        allProfiles.find(profile => profile.id === guest.user_id)?.name || null,
                        async () => {
                            await supabase.from('guest_invites')
                                .delete()
                                .eq('start_date', guest.start_date)
                                .eq('start_hour', guest.start_hour)
                                .eq('guest_name', guest.guest_name)

                            guestHandler.setState(await fetchGuests())
                        }
                    )}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const supabase = createPagesBrowserClient<Database>()
    const {data: location} = await supabase.from('locations')
        .select('*')
        .eq('name', LocationName.Gara)
        .limit(1)
        .single()

    const props: IParams = {
        location: location!
    }

    return {props}
}
