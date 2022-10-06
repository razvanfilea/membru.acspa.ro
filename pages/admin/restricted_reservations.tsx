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
    Stack,
    TextInput,
    Title
} from "@mantine/core";
import React, {useEffect, useState} from "react";
import {useRouter} from "next/router";
import {useAuth} from "../../components/AuthProvider";
import {MemberTypes, ReservationRestriction} from "../../types/wrapper";
import {MdAdd, MdRefresh} from "react-icons/md";
import {supabase} from "../../utils/supabase_utils";
import ReservationRestrictionComponent from "../../components/ReservationRestriction";
import {useForm} from "@mantine/form";
import {DatePicker} from "@mantine/dates";
import {dateToISOString} from "../../utils/date";

const hoursSelection = [...Array(23).keys()].map((_, index) => {
    return {
        key: (index + 1).toString(),
        value: `${index + 1}`
    }
});

export default function RestrictedReservations() {
    const router = useRouter()
    const auth = useAuth()

    const [restrictions, setRestrictions] = useState<ReservationRestriction[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const newRestrictionForm = useForm({
        initialValues: {
            date: new Date(),
            startHour: 7,
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
        if (!auth.isLoading && auth.user == null || auth.profile?.member_type !== MemberTypes.Fondator)
            router.push('/').then(() => {
            })
    }, [auth, router])

    useEffect(() => {
        if (auth.user == null)
            return;

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

                    <NumberInput
                        {...newRestrictionForm.getInputProps('startHour')}
                        placeholder="Ora"
                        label="Ora"
                        required={true}
                        min={7}
                        max={23}
                    />

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
