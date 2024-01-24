import 'dayjs/locale/ro';
import {Button, Card, Modal, Stack, Switch, TextInput} from "@mantine/core";
import {useMemo, useState} from "react";
import {Location, LocationName, ReservationRestriction} from "../../types/wrapper";
import ReservationRestrictionComponent from "../../components/ReservationRestriction";
import {useForm} from "@mantine/form";
import {DatePickerInput} from "@mantine/dates";
import {dateToISOString, isFreeDay} from "../../utils/date";
import {AdminHourInput, AdminTopBar} from "../../components/AdminInput";
import {Database} from "../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {createPagesBrowserClient} from "@supabase/auth-helpers-nextjs";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import useProfilesQuery from "../../hooks/useProfilesQuery";
import useRestrictionsQuery from "../../hooks/useRestrictionsQuery";
import useFreeDaysQuery from "../../hooks/useFreeDaysQuery";
import AdminScaffold from "../../components/AdminInput/AdminScaffold";

interface IParams {
    location: Location
}

export default function RestrictedReservationsList(params: IParams) {
    useExitIfNotFounder();

    const supabase = useSupabaseClient<Database>()
    const game_location = params.location

    const {data: allProfiles} = useProfilesQuery()
    const {data: restrictions, refetch} = useRestrictionsQuery()
    const {data: freeDays} = useFreeDaysQuery(new Date)
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const newRestrictionForm = useForm({
        initialValues: {
            date: new Date,
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

    const hasSelectedWeekend = useMemo(
        () => isFreeDay(newRestrictionForm.values.date, freeDays || []),
        [newRestrictionForm.values.date, freeDays])

    return <>
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
                    newRestrictionForm.reset()
                    await refetch()
                })}>

                <Stack>

                    <DatePickerInput
                        {...newRestrictionForm.getInputProps('date')}
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

        <AdminScaffold>
            <AdminTopBar
                title={'Rezervările blocate:'}
                onAdd={() => setCreateModalOpened(true)}/>

            {restrictions?.map((restriction) => (
                <Card key={restriction.date + restriction.start_hour} shadow={"xs"}>
                    {ReservationRestrictionComponent(
                        restriction,
                        allProfiles?.find(profile => profile.id === restriction.user_id)?.name || null,
                        async () => {
                            await supabase.from('reservations_restrictions')
                                .delete()
                                .eq('date', restriction.date)
                                .eq('start_hour', restriction.start_hour)

                            await refetch()
                        }
                    )}
                </Card>
            ))}
        </AdminScaffold>
    </>
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
