import React, {ReactElement, useEffect, useMemo, useState} from "react";
import Head from "next/head";
import {ActionIcon, Grid, Group, Paper, Space, Stack, Text, Title} from "@mantine/core";
import {useListState, useScrollIntoView} from "@mantine/hooks";
import 'dayjs/locale/ro'
import {Location, LocationName, Reservation} from "../types/wrapper";
import {useRouter} from "next/router";
import {MdRefresh} from "react-icons/md";
import {addDaysToDate, dateToISOString, isDateWeekend} from "../utils/date";
import {ConfirmSelection, GeneralInfoPopup, RegistrationHours} from "../components/MainPageComponents";
import {SupabaseClient, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {createPagesBrowserClient} from "@supabase/auth-helpers-nextjs";
import {DatePicker} from "@mantine/dates";
import useProfileData from "../hooks/useProfileData";
import useRestrictionsQuery from "../hooks/useRestrictionsQuery";
import useGuestsQuery from "../hooks/useGuestsQuery";

interface IParams {
    gara: Location
    boromir: Location
    daysAhead: number
}

export default function MakeReservationPage(params: IParams): ReactElement {
    const router = useRouter()
    const profileData = useProfileData()

    const [locationName, /*setLocationName*/] = useState(LocationName.Gara)
    const [selectedDate, setSelectedDate] = useState<Date>(new Date)
    const [selectedTable, setSelectedTable] = useState<number | null>(null)

    const selectedDateISO = useMemo(() => dateToISOString(selectedDate), [selectedDate])

    function onSelectedDateChange(selectedDate: Date) {
        setSelectedDate(selectedDate)
        setSelectedTable(null)
    }

    useEffect(() => {
        if (!profileData.isLoading && profileData.profile == null) {
            const timer = setTimeout(() => {
                router.push('/login').then(null)
            }, 400)

            return () => clearTimeout(timer)
        }
    }, [profileData, router])

    const location = locationName == LocationName.Gara ? params.gara : params.boromir;

    return <>
        <Head>
            <title>Rezervări - ACSPA</title>
        </Head>

        <Title>Rezervări</Title>

        <Space h="lg"/>

        <Paper>
            <GeneralInfoPopup/>

            <Grid
                grow={true}
                columns={4}>

                <Grid.Col span={'auto'}>
                    <Text>Alege ziua rezervării:</Text>

                    {!profileData.isLoading && profileData.profile != null &&
                        <DatePicker
                            minDate={new Date}
                            maxDate={addDaysToDate(new Date, params.daysAhead)}
                            hideOutsideDates={true}
                            maxLevel={'month'}
                            size={"lg"}
                            locale={"ro"}
                            value={selectedDate}
                            onChange={(date) => {
                                if (profileData.profile != null && date != null)
                                    onSelectedDateChange(date)
                            }}
                            getDayProps={(date) => {
                                if (date.getDate() === (new Date).getDate()
                                    && date.getMonth() === (new Date).getMonth()
                                    && date.getDate() !== selectedDate?.getDate()) {
                                    return {
                                        sx: (theme) => ({
                                            backgroundColor: theme.colors.blue[7],
                                            color: theme.white
                                        })
                                    };
                                }
                                return {};
                            }}
                            withCellSpacing={true}
                        />
                    }
                </Grid.Col>

                <Grid.Col span={2}>
                    <Stack>
                        {SelectGameTable(location, selectedDateISO, selectedTable, setSelectedTable)}

                        {ConfirmSelection(location, selectedDateISO, selectedTable)}
                    </Stack>
                </Grid.Col>
            </Grid>

            <Space h="xl"/>
        </Paper>
    </>;
}

function fetchReservations(
    supabase: SupabaseClient<Database>,
    setReservations: (data: Reservation[]) => void
) {
    supabase.from('rezervari')
        .select('*')
        .gte('start_date', dateToISOString(new Date))
        .eq('cancelled', false)
        .order('created_at', {ascending: true})
        .then(value => {
            if (value.data != null) {
                setReservations(value.data)
                console.log("Reservations updated")
            }
        })
}

function SelectGameTable(
    location: Location,
    selectedDateISO: string,
    selectedStartHour: number | null,
    onSetStartHour: (s: number) => void,
): ReactElement {
    const supabase = useSupabaseClient<Database>()
    const router = useRouter()

    const [reservations, reservationsHandle] = useListState<Reservation>([])
    const {data: restrictions} = useRestrictionsQuery(new Date)
    const {data: guests} = useGuestsQuery(new Date)
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        fetchReservations(supabase, reservationsHandle.setState);

        const reservationListener = supabase.channel('rezervari')
            .on(
                'postgres_changes',
                {event: '*', schema: 'public', table: 'rezervari'},
                (payload) => {
                    if (payload.eventType == "INSERT") {
                        if (payload.new.cancelled === false) {
                            reservationsHandle.setState((prev) => {
                                    return [...prev, payload.new as Reservation]
                                }
                            )
                        }
                    } else if (payload.eventType == "UPDATE") {
                        fetchReservations(supabase, reservationsHandle.setState) // TODO Could make this more efficient
                    } else {
                        reservationsHandle.filter(value => value.id != payload.old.id)
                    }
                }
            )
            .subscribe()

        return () => {
            reservationListener?.unsubscribe()
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    useEffect(() => scrollIntoView({alignment: 'center'}), [scrollIntoView, selectedDateISO])

    const registrationHours = useMemo(() => {
        if (selectedDateISO == null) {
            return {start: 0, end: 0, duration: 0}
        }

        if (isDateWeekend(new Date(selectedDateISO))) {
            return {
                start: location.weekend_start_hour,
                end: location.weekend_end_hour,
                duration: location.weekend_reservation_duration,
            }
        }

        return {
            start: location.start_hour,
            end: location.end_hour,
            duration: location.reservation_duration,
        }
    }, [location, selectedDateISO])

    const selectedDateReservations = useMemo(
        () => reservations.filter(value => value.start_date == selectedDateISO),
        [reservations, selectedDateISO])
    const selectedDateInvites = useMemo(
        () => guests?.filter(value => value.start_date == selectedDateISO) || [],
        [guests, selectedDateISO])
    const selectedDateRestrictions = useMemo(
        () => restrictions?.filter(value => value.date == selectedDateISO) || [],
        [restrictions, selectedDateISO])

    return <>
        <Group position={'apart'} align={"center"}>
            <Group align={"center"} spacing={'xs'}>
                <Text weight={600} ref={targetRef}>Data selectată:</Text>
                <Text color={"blue"}>{new Date(selectedDateISO).toLocaleDateString('ro-RO')}</Text>
            </Group>

            <ActionIcon
                variant={'light'} radius={'xl'} size={36}
                onClick={() => router.reload()}>
                <MdRefresh size={28}/>
            </ActionIcon>
        </Group>

        {RegistrationHours(selectedDateReservations, selectedDateRestrictions, selectedDateInvites, selectedStartHour, onSetStartHour, registrationHours)}
    </>
}

export async function getStaticProps({}) {
    const supabase = createPagesBrowserClient<Database>()

    const {data: locations} = await supabase.from('locations').select()
    const garaLocation = locations!.find(value => value.name == LocationName.Gara)
    const boromirLocation = locations!.find(value => value.name == LocationName.Boromir)

    const props: IParams = {
        daysAhead: 14,
        gara: garaLocation!,
        boromir: boromirLocation!
    }

    return {props}
}
