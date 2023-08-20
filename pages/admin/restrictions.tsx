import 'dayjs/locale/ro';
import {Button, Card, Modal, NumberInputHandlers, Stack, Switch, TextInput} from "@mantine/core";
import React, {useEffect, useMemo, useRef, useState} from "react";
import {Location, LocationName, ReservationRestriction} from "../../types/wrapper";
import ReservationRestrictionComponent from "../../components/ReservationRestriction";
import {useForm} from "@mantine/form";
import {DatePickerInput} from "@mantine/dates";
import {dateToISOString, isDateWeekend} from "../../utils/date";
import {AdminHourInput, AdminTopBar} from "../../components/AdminInput";
import {Database} from "../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {createPagesBrowserClient} from "@supabase/auth-helpers-nextjs";
import {useExitIfNotFounder} from "../../utils/admin_tools";
import useProfilesQuery from "../../hooks/useProfilesQuery";

interface IParams {
    location: Location
}

export default function RestrictedReservationsList(params: IParams) {
    const supabase = useSupabaseClient<Database>()
    const game_location = params.location

    const {data: allProfiles} = useProfilesQuery()
    const [restrictions, setRestrictions] = useState<ReservationRestriction[]>([])
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const hourInputHandlers = useRef<NumberInputHandlers>();
    const newRestrictionForm = useForm({
        initialValues: {
            date: new Date(),
            allDay: false,
            startHour: 0,
            message: '',
        },

        validate: {
            startHour: (value, values) => value !== 0 || values.allDay ? null : "Ora de început trebuie să fie diferită de 0",
            message: (value) => (value.length >= 5) ? null : "Mesajul este prea scurt",
        },
        validateInputOnBlur: true
    });

    useExitIfNotFounder();

    useEffect(() => {
        fetchRestrictions().then(data => setRestrictions(data))
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

                    if (values.allDay) {
                        const step = hasSelectedWeekend ? game_location.weekend_reservation_duration : game_location.reservation_duration;
                        const min = hasSelectedWeekend ? game_location.weekend_start_hour : game_location.start_hour;
                        const max = hasSelectedWeekend ? (game_location.weekend_end_hour - game_location.weekend_reservation_duration)
                            : (game_location.end_hour - game_location.reservation_duration);

                        let newRestrictions: ReservationRestriction[] = []
                        for (let i = min; i <= max; i += step) {
                            const res = {
                                date: dateToISOString(values.date),
                                start_hour: i,
                                message: values.message
                            }
                            newRestrictions.push(res as ReservationRestriction)
                        }

                        const {error} = await supabase.from('reservations_restrictions').insert(newRestrictions)
                        console.log(error)
                    } else {
                        const newRestriction = {
                            date: dateToISOString(values.date),
                            start_hour: values.startHour,
                            message: values.message
                        }

                        const {error} = await supabase.from('reservations_restrictions').insert([newRestriction])
                        console.log(error)
                    }
                    setRestrictions(await fetchRestrictions())
                    newRestrictionForm.reset()
                })}>

                <Stack>

                    <DatePickerInput
                        {...newRestrictionForm.getInputProps('date')}
                        placeholder="Alege data"
                        label="Data"
                        withAsterisk locale="ro"
                        minDate={new Date()}
                        clearable={false}
                        size={'lg'}
                        dropdownType={'modal'}
                        valueFormat="YYYY-MM-DD"/>

                    <Switch {...newRestrictionForm.getInputProps('allDay')} label="Toată ziua" size={'lg'}/>

                    {!newRestrictionForm.getInputProps('allDay').value &&
                        <AdminHourInput
                            formProps={newRestrictionForm.getInputProps('startHour')}
                            inputHandler={hourInputHandlers}
                            gameLocation={game_location}
                            isWeekend={hasSelectedWeekend}
                        />
                    }

                    <TextInput
                        {...newRestrictionForm.getInputProps('message')}
                        label={'Mesaj'}
                        size={'lg'}
                        placeholder={'Motivul pentru care nu se pot face rezervări'}
                        required={true}/>

                    <Button type={"submit"} color={'green'} px={'sm'}>Adaugă</Button>
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
                onAdd={() => setCreateModalOpened(true)}/>

            {restrictions.map((restriction) => (
                <Card key={restriction.date + restriction.start_hour} shadow={"xs"}>
                    {ReservationRestrictionComponent(
                        restriction,
                        allProfiles?.find(profile => profile.id === restriction.user_id)?.name || null,
                        async () => {
                            await supabase.from('reservations_restrictions')
                                .delete()
                                .eq('date', restriction.date)
                                .eq('start_hour', restriction.start_hour)

                            setRestrictions(await fetchRestrictions())
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
