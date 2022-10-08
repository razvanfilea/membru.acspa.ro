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
import {Location, LocationName, MemberTypes, Profile, ReservationRestriction} from "../../types/wrapper";
import {MdAdd, MdRefresh} from "react-icons/md";
import {supabase} from "../../utils/supabase_utils";
import ReservationRestrictionComponent from "../../components/ReservationRestriction";
import {useForm} from "@mantine/form";
import {DatePicker} from "@mantine/dates";
import {dateToISOString, isWeekend} from "../../utils/date";

interface IParams {
    location: Location
}

export default function RestrictedReservations(params: IParams) {
    const location = params.location
    const router = useRouter()
    const auth = useAuth()

    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const [restrictions, setRestrictions] = useState<ReservationRestriction[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const hourInputHandlers = useRef<NumberInputHandlers>();
    const newRestrictionForm = useForm({
        initialValues: {
            date: new Date(),
            startHour: 0,
            message: '',
        },

        validate: {
            // date: (value) => true,
            // startHour: (value) => (!isNaN(parseInt(value))) ? null : "Număr de oră invalid",
            message: (value) => (value.length >= 10) ? null : "Mesajul este prea scurt",
        },
        validateInputOnChange: true
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

        fetchRestrictions().then(data => setRestrictions(data || []))
        setIsLoading(false)
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [auth.user])

    async function fetchRestrictions() {
        const {data} = await supabase.from<ReservationRestriction>('reservations_restrictions')
            .select('*')
            .order('date', {ascending: true})
            .order('start_hour', {ascending: true})

        return data
    }

    const hasSelectedWeekend = useMemo(() => {
        return isWeekend(newRestrictionForm.values.date)
    }, [newRestrictionForm.values.date])

    if (auth.isLoading || isLoading)
        return <Center> <Loader/> </Center>;

    if (auth.user == null)
        return (<></>)

    return (<>
        <Modal
            opened={createModalOpened}
            onClose={() => setCreateModalOpened(false)}
            title="Restricționează rezervarea"
        >
            <form style={{position: 'relative'}} onSubmit={
                newRestrictionForm.onSubmit(async (values) => {
                    setCreateModalOpened(false)
                    console.log(values.date)
                    const newRestriction = {
                        date: dateToISOString(values.date),
                        start_hour: values.startHour,
                        message: values.message
                    }
                    newRestrictionForm.reset()

                    const {error} = await supabase.from('reservations_restrictions').insert([newRestriction])
                    console.log(error)
                    setRestrictions(await fetchRestrictions() || [])
                })}>

                <Stack>

                    <DatePicker
                        {...newRestrictionForm.getInputProps('date')}
                        placeholder="Alege data"
                        label="Data"
                        withAsterisk locale="ro"
                        inputFormat="YYYY-MM-DD"/>

                    <Group spacing={8} noWrap={true} align={'end'}>
                        <NumberInput
                            {...newRestrictionForm.getInputProps('startHour')}
                            handlersRef={hourInputHandlers}
                            hideControls={true}
                            placeholder="Ora"
                            label="Ora"
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

                    <TextInput
                        {...newRestrictionForm.getInputProps('message')}
                        label={'Mesaj'}
                        placeholder={'Motivul pentru care nu se pot face rezervări'}
                        required={true}/>

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
                <Title order={2}>Rezervările blocate:</Title>

                <Group spacing={'lg'}>
                    <ActionIcon variant={'filled'} color={'green'} radius={'xl'} size={36}
                                onClick={() => setCreateModalOpened(true)}>
                        <MdAdd size={28}/>
                    </ActionIcon>

                    <ActionIcon variant={'filled'} radius={'xl'} size={36} onClick={async () => {
                        setRestrictions(await fetchRestrictions() || [])
                    }}>
                        <MdRefresh size={28}/>
                    </ActionIcon>
                </Group>
            </Group>

            {restrictions.map((reservation) => (
                <Card key={reservation.id} shadow={"xs"}>
                    {ReservationRestrictionComponent(
                        reservation,
                        allProfiles.find(profile => profile.id === reservation.user_id)?.name || null,
                        async () => {
                            await supabase.from<ReservationRestriction>('reservations_restrictions')
                                .delete()
                                .eq('id', reservation.id)
                            setRestrictions(prev => prev.filter(value => value.id !== reservation.id))
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
