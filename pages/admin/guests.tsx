import 'dayjs/locale/ro';
import {Button, Card, Modal, Radio, Stack, TextInput} from "@mantine/core";
import {useMemo, useState} from "react";
import {Location, LocationName} from "../../types/wrapper";
import {useForm} from "@mantine/form";
import {DatePickerInput} from "@mantine/dates";
import {dateToISOString, isFreeDay} from "../../utils/date";
import GuestInviteComponent from "../../components/GuestInvite";
import {AdminHourInput, AdminTopBar} from "../../components/AdminInput";
import {createPagesBrowserClient} from "@supabase/auth-helpers-nextjs";
import {Database} from "../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import useExitIfNotFounder from "../../hooks/useExitIfNotFounder";
import useProfilesQuery from "../../hooks/useProfilesQuery";
import useGuestsQuery from "../../hooks/useGuestsQuery";
import useFreeDaysQuery from "../../hooks/useFreeDaysQuery";
import AdminScaffold from "../../components/AdminInput/AdminScaffold";

interface IParams {
    location: Location
}

export default function GuestManager(params: IParams) {
    useExitIfNotFounder();

    const supabase = useSupabaseClient<Database>()
    const game_location = params.location

    const {data: allProfiles} = useProfilesQuery()
    const {data: guests, refetch: refetchGuests} = useGuestsQuery()
    const {data: freeDays} = useFreeDaysQuery(new Date)
    const [createModalOpened, setCreateModalOpened] = useState(false)

    const newInviteForm = useForm({
        initialValues: {
            date: new Date,
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

    const hasSelectedWeekend = useMemo(
        () => isFreeDay(newInviteForm.values.date, freeDays || []),
        [newInviteForm.values.date, freeDays])

    return <>
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

                    const {error} = await supabase.from('guests').insert([newGuest])
                    if (error != null)
                        console.log(error)
                    await refetchGuests()
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
                        label="Data"
                        withAsterisk locale="ro"
                        minDate={new Date()}
                        clearable={false}
                        size={'lg'}
                        dropdownType={'modal'}
                        valueFormat="YYYY-MM-DD"/>

                    <AdminHourInput
                        formProps={newInviteForm.getInputProps('startHour')}
                        gameLocation={game_location}
                        isWeekend={hasSelectedWeekend}/>

                    <Button type={"submit"} color={'green'}>Adaugă</Button>
                </Stack>
            </form>
        </Modal>

        <AdminScaffold>
            <AdminTopBar
                title={'Invitați:'}
                onAdd={() => setCreateModalOpened(true)}/>

            {guests?.map((guest) => (
                <Card key={guest.start_date + guest.start_hour + guest.guest_name} shadow={"xs"}>
                    {GuestInviteComponent(
                        guest,
                        allProfiles?.find(profile => profile.id === guest.user_id)?.name || null,
                        async () => {
                            await supabase.from('guests')
                                .delete()
                                .eq('start_date', guest.start_date)
                                .eq('start_hour', guest.start_hour)
                                .eq('guest_name', guest.guest_name)

                            await refetchGuests()
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
