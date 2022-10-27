import 'dayjs/locale/ro';
import {
    ActionIcon,
    Button,
    Card,
    Center,
    Group,
    Loader,
    Modal,
    NumberInput,
    NumberInputHandlers,
    Stack,
    TextInput,
    Title
} from "@mantine/core";
import React, {useEffect, useMemo, useRef, useState} from "react";
import {useRouter} from "next/router";
import {useAuth} from "../../components/AuthProvider";
import {GuestInvite, Location, LocationName, MemberTypes, Profile} from "../../types/wrapper";
import {MdAdd, MdRefresh} from "react-icons/md";
import {supabase} from "../../utils/supabase_utils";
import {useForm} from "@mantine/form";
import {DatePicker} from "@mantine/dates";
import {dateToISOString, isWeekend} from "../../utils/date";
import {useListState} from "@mantine/hooks";
import GuestInviteComponent from "../../components/GuestInvite";

interface IParams {
    location: Location
}

export default function GuestManager(params: IParams) {
    const location = params.location
    const router = useRouter()
    const auth = useAuth()

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
        }
    });

    useEffect(() => {
        if ((!auth.isLoading && auth.user == null) || auth.profile?.member_type !== MemberTypes.Fondator)
            router.back()
    }, [auth, router])

    useEffect(() => {
        if (auth.user == null)
            return;

        supabase.from<Profile>('profiles').select('*').then(value => {
            if (value.data != null) {
                setAllProfiles(value.data)
            }
        })

        fetchGuests().then(data => guestHandler.setState(data))
        setIsLoading(false)
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [auth.user])

    async function fetchGuests() {
        const {data} = await supabase.from<GuestInvite>('guest_invites')
            .select('*')
            .order('date', {ascending: true})
            .order('start_hour', {ascending: true})

        return data || []
    }

    const hasSelectedWeekend = useMemo(() => {
        return isWeekend(newInviteForm.values.date)
    }, [newInviteForm.values.date])

    if (auth.isLoading || isLoading)
        return <Center> <Loader/> </Center>;

    if (auth.user == null)
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
                        date: dateToISOString(values.date),
                        start_hour: values.startHour,
                        guest_name: values.guestName
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
                        required={true}/>

                    <DatePicker
                        {...newInviteForm.getInputProps('date')}
                        placeholder="Alege data"
                        label="Data"
                        withAsterisk locale="ro"
                        inputFormat="YYYY-MM-DD"/>

                    <Group spacing={8} noWrap={true} align={'end'}>
                        <NumberInput
                            {...newInviteForm.getInputProps('startHour')}
                            handlersRef={hourInputHandlers}
                            hideControls={true}
                            placeholder="Ora"
                            label="Ora"
                            disabled={true}
                            required={true}
                            step={hasSelectedWeekend ? location.weekend_reservation_duration : location.reservation_duration}
                            min={hasSelectedWeekend ? location.weekend_start_hour : location.start_hour}
                            max={hasSelectedWeekend ? (location.weekend_end_hour - location.weekend_reservation_duration)
                                : (location.end_hour - location.reservation_duration)}
                        />
                        <ActionIcon size={36} variant="default"
                                    onClick={() => hourInputHandlers.current!.decrement()}>
                            –
                        </ActionIcon>
                        <ActionIcon size={36} variant="default"
                                    onClick={() => hourInputHandlers.current!.increment()}>
                            +
                        </ActionIcon>
                    </Group>

                    <Button type={"submit"}>Submit</Button>

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
            <Group position={'apart'}>
                <Title order={2}>Invitații:</Title>

                <Group spacing={'lg'}>
                    <ActionIcon variant={'filled'} color={'green'} radius={'xl'} size={36}
                                onClick={() => setCreateModalOpened(true)}>
                        <MdAdd size={28}/>
                    </ActionIcon>

                    <ActionIcon variant={'filled'} radius={'xl'} size={36} onClick={async () => {
                        guestHandler.setState(await fetchGuests())
                    }}>
                        <MdRefresh size={28}/>
                    </ActionIcon>
                </Group>
            </Group>

            {guests.map((guest) => (
                <Card key={guest.date + guest.start_hour} shadow={"xs"}>
                    {GuestInviteComponent(
                        guest,
                        allProfiles.find(profile => profile.id === guest.user_id)?.name || null,
                        async () => {
                            await supabase.from<GuestInvite>('guest_invites')
                                .delete()
                                .eq('date', guest.date)
                                .eq('start_hour', guest.start_hour)
                            guestHandler.filter(value => value.date !== guest.date && value.start_hour !== guest.start_hour)
                        }
                    )}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const {data: location} = await supabase.from<Location>('locations')
        .select('*')
        .eq('name', LocationName.Gara)
        .limit(1)
        .single()

    const props: IParams = {
        location: location!
    }

    return {props}
}
