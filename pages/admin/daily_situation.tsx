import {useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";
import {useProfile} from "../../components/ProfileProvider";
import React, {useEffect, useMemo, useState} from "react";
import {Profile, Reservation, ReservationStatus} from "../../types/wrapper";
import {Button, Center, Divider, Grid, Group, Loader, Space, Stack, Text} from "@mantine/core";
import {DatePicker} from "@mantine/dates";
import 'dayjs/locale/ro';
import {dateToISOString} from "../../utils/date";
import {useExitIfNotFounder} from "../../utils/admin_tools";

const groupBy = <T, K extends keyof any>(arr: T[], key: (i: T) => K) =>
    arr.reduce((groups, item) => {
        (groups[key(item)] ||= []).push(item);
        return groups;
    }, {} as Record<K, T[]>);

export default function DailySituationPage() {
    const supabase = useSupabaseClient<Database>()
    const profileData = useProfile()

    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const [reservations, setReservations] = useState<Reservation[]>([])
    const [isLoading, setIsLoading] = useState(true)
    const [date, setDate] = useState<Date | undefined>(undefined);

    useExitIfNotFounder();

    useEffect(() => {
        supabase.from('profiles').select('*')
            .order('name', {ascending: true})
            .then(value => {
                if (value.data != null) {
                    setAllProfiles(value.data)
                    setIsLoading(false)
                }
            })
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    useEffect(() => {
        if (date) {
            supabase.from('rezervari').select('*')
                .eq('status', ReservationStatus.Approved)
                .eq('start_date', dateToISOString(date))
                .then(value => {
                    if (value.data != null) {
                        setReservations(value.data)
                    }
                })
        }
    }, [date, supabase])

    const groupedReservations = useMemo(() => {
        return groupBy(reservations, reservation => reservation.start_hour)
    }, [reservations])

    if (profileData.isLoading || isLoading)
        return <Center> <Loader/> </Center>;

    if (profileData.profile == null)
        return (<></>)

    return <>
        <Grid
            grow={true}
            columns={4}
        >

            <Grid.Col span={"auto"}>
                <DatePicker
                    value={date}
                    onChange={(newDate) => setDate(newDate || undefined)}
                    size={"lg"}
                    locale="ro"
                />
            </Grid.Col>

            <Grid.Col span={2}>
                <Stack p={'md'}>
                    {date &&
                        SelectedDateReservations(allProfiles, groupedReservations)
                    }

                    {!date &&
                        <Text size={'xl'}>Selectează o dată pentru a vedea rezervările</Text>
                    }
                </Stack>
            </Grid.Col>
        </Grid>

        <Space h="xl"/>
    </>
}

function SelectedDateReservations(allProfiles: Profile[], reservations: Record<number, Reservation[]>) {

    return <>
        {Object.entries(reservations).map(([key, reservation]) => {
            return <React.Fragment key={key}>
                <Group>
                    <Text><b>Ora {key}:</b></Text>

                    {
                        reservation.map((user, index) => {
                            const profile = allProfiles.find(profile => profile.id == user.user_id) || {
                                name: 'Necunoscut',
                                has_key: false
                            }
                            return <Button key={user.user_id} color={profile.has_key ? 'blue' : 'gray'} radius={'xl'}
                                           size={'xs'}>{index + 1}. {profile.name}</Button>
                        })
                    }
                </Group>

                <Divider mt={'md'} mb={'md'}/>
            </React.Fragment>
        })
        }
    </>
}
