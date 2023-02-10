import 'dayjs/locale/ro';
import {Button, Card, Center, Loader, Modal, NumberInputHandlers, Stack, TextInput} from "@mantine/core";
import React, {useEffect, useMemo, useRef, useState} from "react";
import {useProfile} from "../../components/ProfileProvider";
import {Location, LocationName, Profile, ReservationRestriction} from "../../types/wrapper";
import ReservationRestrictionComponent from "../../components/ReservationRestriction";
import {useForm} from "@mantine/form";
import {DatePicker} from "@mantine/dates";
import {dateToISOString, isDateWeekend} from "../../utils/date";
import {AdminHourInput, AdminTopBar} from "../../components/AdminInput";
import {Database} from "../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {createBrowserSupabaseClient} from "@supabase/auth-helpers-nextjs";
import {useExitIfNotFounder} from "../../utils/admin_tools";

interface IParams {
    location: Location
}

export default function RestrictedReservationsList(params: IParams) {
    const supabase = useSupabaseClient<Database>()
    const location = params.location
    const profileData = useProfile()

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
            startHour: (value) => value !== 0 ? null : "Ora de început trebuie să fie diferită de 0",
            message: (value) => (value.length >= 5) ? null : "Mesajul este prea scurt",
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

        fetchRestrictions().then(data => setRestrictions(data))
        setIsLoading(false)
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    async function fetchRestrictions() {
        const {data} = await supabase.from('reservations_restrictions')
            .select('*')
            .order('date', {ascending: false})
            .order('start_hour', {ascending: true})

        return data || []
    }

    const hasSelectedWeekend = useMemo(() => {
        return isDateWeekend(newRestrictionForm.values.date)
    }, [newRestrictionForm.values.date])

    if (profileData.isLoading || isLoading)
        return <Center> <Loader/> </Center>;

    if (profileData.profile == null)
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
                    setRestrictions(await fetchRestrictions())
                })}>

                <Stack>

                    <DatePicker
                        {...newRestrictionForm.getInputProps('date')}
                        placeholder="Alege data"
                        label="Data"
                        withAsterisk locale="ro"
                        minDate={new Date()}
                        clearable={false}
                        size={'lg'}
                        inputFormat="YYYY-MM-DD"/>

                    <AdminHourInput
                        formProps={newRestrictionForm.getInputProps('startHour')}
                        inputHandler={hourInputHandlers}
                        gameLocation={location}
                        isWeekend={hasSelectedWeekend}
                    />

                    <TextInput
                        {...newRestrictionForm.getInputProps('message')}
                        label={'Mesaj'}
                        size={'lg'}
                        placeholder={'Motivul pentru care nu se pot face rezervări'}
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
            <AdminTopBar
                title={'Rezervările blocate:'}
                onAdd={() => setCreateModalOpened(true)}
                onRefresh={async () => setRestrictions(await fetchRestrictions())}/>

            {restrictions.map((restriction) => (
                <Card key={restriction.date + restriction.start_hour} shadow={"xs"}>
                    {ReservationRestrictionComponent(
                        restriction,
                        allProfiles.find(profile => profile.id === restriction.user_id)?.name || null,
                        async () => {
                            await supabase.from('reservations_restrictions')
                                .delete()
                                .eq('date', restriction.date)
                                .eq('start_hour', restriction.start_hour)
                            setRestrictions(prev => prev.filter(value => value.date !== restriction.date && value.start_hour !== restriction.start_hour))
                        }
                    )}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const supabase = createBrowserSupabaseClient<Database>()
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
